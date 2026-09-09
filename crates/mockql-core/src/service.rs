// Copyright 2026 Expedia, Inc.
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     https://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

//! Planning and execution entry points for mock-aware GraphQL requests.

use crate::Header;
use crate::graphql::GraphQLResponse;
use crate::graphql::mock_response_prompt::MockResponsePrompt;
use crate::graphql::mock_response_prompt::SerializationFormat;
use crate::graphql::response_merger::ResponseMerger;
use crate::graphql::response_merger::ResponseMergerError;
use crate::graphql::response_validator::ResponseValidationError;
use crate::graphql::response_validator::ResponseValidator;
use crate::llm_provider::ProviderConfig;
use crate::llm_provider::ProviderError;
use crate::llm_provider::generate_mock_response;
use crate::planner::MockPlanner;
use crate::planner::SplitError;
use crate::planner::SplitResult;
use crate::upstream::UpstreamError;
use crate::upstream::execute_graphql;
use apollo_compiler::ExecutableDocument;
use apollo_compiler::Schema;
use apollo_compiler::response::ExecutionResponse;
use apollo_compiler::validation::Valid;
use reqwest::Client;
use reqwest::Url;
use serde_json_bytes::ByteString;
use serde_json_bytes::Map;
use serde_json_bytes::Value;
use std::sync::Arc;
use std::time::Duration;
use thiserror::Error;

const MAX_VALIDATION_RETRIES: usize = 3;
const MAX_PROVIDER_RETRIES: usize = 3;
const PROVIDER_RETRY_DELAY: Duration = Duration::from_secs(1);

/// Execution strategy selected for a request after planning.
#[derive(Debug, Clone)]
pub enum MockPlan {
  /// The operation contains no mocks and should be executed upstream.
  NoMocks,
  /// The operation is a pure introspection query answered from the local schema.
  Introspection(ExecutionResponse),
  /// The operation should be handled entirely by a mock provider.
  FullMock(MockResponsePrompt),
  /// The operation should combine upstream data with generated mock data.
  PartialMock {
    /// Prompt used to ask the provider for the mocked portion.
    prompt: MockResponsePrompt,
    /// Upstream operation to execute before merging in mock data.
    upstream_operation: String,
  },
}

impl From<SplitResult> for MockPlan {
  fn from(split_result: SplitResult) -> Self {
    match split_result {
      SplitResult::NoMocks => MockPlan::NoMocks,
      SplitResult::Introspection(response) => MockPlan::Introspection(response),
      SplitResult::FullMock { prompt, .. } => MockPlan::FullMock(prompt),
      SplitResult::PartialMock {
        prompt,
        upstream_operation,
        ..
      } => MockPlan::PartialMock {
        prompt,
        upstream_operation,
      },
    }
  }
}

/// Inputs required to fully execute a request.
#[derive(Debug, Clone)]
pub struct MockServiceRequest {
  /// GraphQL operation text.
  pub operation: String,
  /// Optional operation name for multi-operation documents.
  pub operation_name: Option<String>,
  /// Variables supplied to the operation.
  pub variables: Map<ByteString, Value>,
  /// Optional schema extension payload applied before planning.
  pub schema_extension: Option<Value>,
  /// Upstream GraphQL server URL for passthrough execution.
  pub graphql_url: Url,
  /// Headers sent to the upstream GraphQL server.
  pub graphql_headers: Vec<Header>,
  /// Provider configuration used for mock generation.
  pub provider: ProviderConfig,
}

#[buildstructor::buildstructor]
impl MockServiceRequest {
  /// Constructs a new [`MockServiceRequest`].
  #[builder(visibility = "pub")]
  fn new(
    operation: String,
    operation_name: Option<String>,
    variables: Map<ByteString, Value>,
    schema_extension: Option<Value>,
    graphql_url: Url,
    graphql_headers: Vec<Header>,
    provider: ProviderConfig,
  ) -> Self {
    Self {
      operation,
      operation_name,
      variables,
      schema_extension,
      graphql_url,
      graphql_headers,
      provider,
    }
  }
}

/// Plans and executes GraphQL operations that may contain `@mock` directives.
#[derive(Debug)]
pub struct MockService {
  planner: MockPlanner,
  client: Client,
}

/// Errors raised while planning or executing a request.
#[derive(Debug, Error)]
pub enum ServiceError {
  /// Planning failed before any external I/O occurred.
  #[error(transparent)]
  Split(#[from] SplitError),
  /// Mock provider execution failed.
  #[error(transparent)]
  Provider(#[from] ProviderError),
  /// Upstream GraphQL execution failed.
  #[error(transparent)]
  Upstream(#[from] UpstreamError),
  /// Partial-response merging failed.
  #[error(transparent)]
  Merge(#[from] ResponseMergerError),
  /// Generated response validation failed.
  #[error(transparent)]
  ResponseValidation(#[from] ResponseValidationError),
}

fn is_retryable_provider_error(error: &ProviderError) -> bool {
  match error {
    ProviderError::Http(error) => error.is_timeout() || error.is_connect() || error.is_body(),
    ProviderError::UnexpectedStatus { status, .. } => {
      *status == reqwest::StatusCode::REQUEST_TIMEOUT
        || *status == reqwest::StatusCode::TOO_MANY_REQUESTS
        || status.is_server_error()
    }
    _ => false,
  }
}

async fn generate_and_validate_mock_response(
  provider: &ProviderConfig,
  mut prompt: MockResponsePrompt,
  validator: &ResponseValidator,
  document: &Valid<ExecutableDocument>,
  operation_name: Option<&str>,
  variables: &Map<ByteString, Value>,
) -> Result<GraphQLResponse, ServiceError> {
  let mut provider_retries = 0;
  let mut validation_retries = 0;
  loop {
    let mut response = match generate_mock_response(provider, &prompt).await {
      Ok(response) => response,
      Err(error) if is_retryable_provider_error(&error) => {
        if provider_retries >= MAX_PROVIDER_RETRIES {
          return Err(error.into());
        }

        provider_retries += 1;
        if std::env::var("MOCKQL_DEBUG").is_ok() {
          eprintln!("provider failed; retrying ({provider_retries}/{MAX_PROVIDER_RETRIES}): {error}");
        }
        tokio::time::sleep(PROVIDER_RETRY_DELAY).await;
        continue;
      }
      Err(error) => return Err(error.into()),
    };
    if std::env::var("MOCKQL_DEBUG").is_ok() {
      eprintln!(
        "LLM response: {}",
        serde_json::to_string(&response).unwrap_or_else(|_| format!("{response:?}"))
      );
    }
    let validation = response
      .data
      .as_ref()
      .ok_or(ResponseValidationError::InvalidData)
      .and_then(|data| validator.validate(document, operation_name, variables, data));

    match validation {
      Ok(data) => {
        response.data = Some(data);
        return Ok(response);
      }
      Err(error) => {
        if validation_retries >= MAX_VALIDATION_RETRIES {
          return Err(error.into());
        }

        let feedback = match &error {
          ResponseValidationError::Execution(errors) => serde_json::to_string(errors)
            .unwrap_or_else(|_| serde_json::json!({ "message": error.to_string() }).to_string()),
          ResponseValidationError::InvalidData => serde_json::json!({ "message": error.to_string() }).to_string(),
          ResponseValidationError::InvalidRequest(_) => return Err(error.into()),
        };

        validation_retries += 1;
        if std::env::var("MOCKQL_DEBUG").is_ok() {
          eprintln!(
            "mock response validation failed; retrying ({validation_retries}/{MAX_VALIDATION_RETRIES}): {error}"
          );
        }
        prompt = prompt.with_validation_feedback(feedback);
      }
    }
  }
}

impl MockService {
  /// Creates a new service for a validated schema and shared upstream client.
  pub fn new(schema: Arc<Valid<Schema>>, serialization_format: SerializationFormat, client: Client) -> Self {
    Self {
      planner: MockPlanner::new(schema, serialization_format),
      client,
    }
  }

  /// Builds a [`MockPlan`].
  pub fn plan(
    &self,
    operation: &str,
    operation_name: Option<&str>,
    variables: &Map<ByteString, Value>,
    schema_extension: Option<&Value>,
  ) -> Result<MockPlan, ServiceError> {
    Ok(
      self
        .planner
        .split_operation(operation, operation_name, variables, schema_extension)?
        .into(),
    )
  }

  /// Executes a request using upstream GraphQL, a mock provider, or both.
  pub async fn execute(&self, request: MockServiceRequest) -> Result<GraphQLResponse, ServiceError> {
    let plan = self.planner.split_operation(
      request.operation.as_str(),
      request.operation_name.as_deref(),
      &request.variables,
      request.schema_extension.as_ref(),
    )?;

    match plan {
      SplitResult::Introspection(response) => Ok(GraphQLResponse {
        data: response.data.map(Value::Object),
        errors: response
          .errors
          .into_iter()
          .filter_map(|e| serde_json_bytes::to_value(e).ok())
          .collect(),
        extensions: Map::new(),
      }),
      SplitResult::NoMocks => execute_graphql(
        &self.client,
        request.graphql_url,
        &request.graphql_headers,
        &request.operation,
        request.operation_name.as_deref(),
        &request.variables,
      )
      .await
      .map_err(ServiceError::from),
      SplitResult::FullMock {
        prompt,
        validator,
        document,
      } => {
        generate_and_validate_mock_response(
          &request.provider,
          prompt,
          &validator,
          &document,
          request.operation_name.as_deref(),
          &request.variables,
        )
        .await
      }
      SplitResult::PartialMock {
        prompt,
        validator,
        document,
        original_document,
        upstream_operation: passthrough_operation,
      } => {
        let mut upstream_response = execute_graphql(
          &self.client,
          request.graphql_url,
          &request.graphql_headers,
          &passthrough_operation,
          request.operation_name.as_deref(),
          &request.variables,
        )
        .await?;
        if upstream_response.data.as_ref().is_none_or(Value::is_null) {
          return Ok(upstream_response);
        }
        let contextual_prompt = prompt.with_partial_response(upstream_response.data.clone());
        let mut mock_response = generate_and_validate_mock_response(
          &request.provider,
          contextual_prompt,
          &validator,
          &document,
          request.operation_name.as_deref(),
          &request.variables,
        )
        .await?;
        ResponseMerger::merge(&mut upstream_response, &mut mock_response)?;
        upstream_response.data = Some(
          validator.validate(
            &original_document,
            request.operation_name.as_deref(),
            &request.variables,
            upstream_response
              .data
              .as_ref()
              .ok_or(ResponseValidationError::InvalidData)?,
          )?,
        );
        Ok(upstream_response)
      }
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use serde_json_bytes::Map;

  fn test_service(schema_sdl: &str) -> MockService {
    let schema = Schema::builder()
      .parse(
        "directive @mock(hint: String) on QUERY | MUTATION | FIELD",
        "mock-directive.graphql",
      )
      .parse(schema_sdl, "schema.graphql")
      .build()
      .unwrap();
    let schema = Arc::new(schema.validate().unwrap());
    let client = Client::builder().timeout(Duration::from_secs(30)).build().unwrap();
    MockService::new(schema, Default::default(), client)
  }

  #[test]
  fn new_builds_service() {
    let service = test_service("type Query { hello: String }");
    assert!(matches!(
      service.plan("{ hello }", None, &Map::new(), None),
      Ok(MockPlan::NoMocks)
    ));
  }

  #[test]
  fn plan_without_mocks_returns_no_mocks() {
    let service = test_service("type Query { hello: String }");
    let plan = service
      .plan("{ hello }", None, &Map::new(), None)
      .expect("plan should succeed");
    assert!(matches!(plan, MockPlan::NoMocks));
  }

  #[test]
  fn plan_full_mock_returns_full_mock() {
    let service = test_service("type Query { hello: String }");
    let plan = service
      .plan("{ hello @mock }", None, &Map::new(), None)
      .expect("plan should succeed");
    assert!(matches!(plan, MockPlan::FullMock(_)));
  }

  #[test]
  fn plan_partial_mock_returns_partial_mock() {
    let service = test_service(
      r"
        type Query { me: User }
        type User { id: ID name: String }
      ",
    );
    let plan = service
      .plan("{ me { id name @mock } }", None, &Map::new(), None)
      .expect("plan should succeed");
    assert!(matches!(plan, MockPlan::PartialMock { .. }));
  }

  #[test]
  fn plan_introspection_returns_introspection() {
    let service = test_service("type Query { hello: String }");
    let plan = service
      .plan("{ __schema { queryType { name } } }", None, &Map::new(), None)
      .expect("plan should succeed");
    assert!(matches!(plan, MockPlan::Introspection(_)));
  }
}
