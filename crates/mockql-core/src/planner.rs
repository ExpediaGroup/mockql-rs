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

//! Planning helpers for mock-aware GraphQL requests.

use crate::MinifyExt;
use crate::graphql::mock_directive_extractor::MockDirectiveExtractor;
use crate::graphql::mock_directive_extractor::MockDirectiveExtractorError;
use crate::graphql::mock_directive_extractor::MockScenario;
use crate::graphql::mock_response_prompt::MockResponsePrompt;
use crate::graphql::mock_response_prompt::SerializationFormat;
use crate::graphql::operation_field_filter::FilterMode;
use crate::graphql::operation_field_filter::OperationFieldFilter;
use crate::graphql::schema_filter::SchemaFilter;
use crate::graphql::schema_filter::SchemaFilterError;
use apollo_compiler::Schema;
use apollo_compiler::parser::FileId;
use apollo_compiler::response::ExecutionResponse;
use apollo_compiler::validation::Valid;
use serde_json_bytes::ByteString;
use serde_json_bytes::Map;
use serde_json_bytes::Value;
use std::sync::Arc;
use thiserror::Error;

#[derive(Debug)]
pub(crate) struct MockPlanner {
  schema: Arc<Valid<Schema>>,
  schema_filter: SchemaFilter,
  mock_directive_extractor: MockDirectiveExtractor,
  serialization_format: SerializationFormat,
}

/// Internal result of splitting an operation into passthrough and mocked portions.
#[derive(Debug)]
pub enum SplitResult {
  /// The operation contains no mocks and should be forwarded upstream as-is.
  NoMocks,
  /// The operation is a pure introspection query answered from the local schema.
  Introspection(ExecutionResponse),
  /// The full operation should be satisfied by a mock provider.
  FullMock(MockResponsePrompt),
  /// The operation should be split between upstream execution and provider generation.
  PartialMock {
    /// Prompt describing the portion that should be mocked.
    prompt: MockResponsePrompt,
    /// GraphQL operation that should be executed upstream before merging.
    upstream_operation: String,
  },
}

/// Errors raised while analyzing how an operation should be split.
#[derive(Debug, Error)]
pub enum SplitError {
  /// `@mock` directive extraction failed.
  #[error("failed to extract mock directives: {0}")]
  MockExtraction(#[from] MockDirectiveExtractorError),
  /// The operation could not be filtered into include or exclude variants.
  #[error("failed to filter operation fields: {0}")]
  OperationFilter(String),
  /// The schema could not be minimized for the target operation.
  #[error("failed to filter schema: {0}")]
  SchemaFilter(#[from] SchemaFilterError),
  /// A provided schema extension was invalid.
  #[error("schema extension failed: {0}")]
  SchemaExtension(#[from] ExtendSchemaError),
  /// The schema extension input value was invalid.
  #[error("invalid schema extension: {0}")]
  SchemaExtensionInput(String),
}

#[derive(Debug, Error)]
pub enum ExtendSchemaError {
  #[error("failed to build extended schema: {0:?}")]
  Build(Vec<String>),
  #[error("failed to validate extended schema: {0:?}")]
  Validate(Vec<String>),
}

impl MockPlanner {
  pub(crate) fn new(schema: Arc<Valid<Schema>>, serialization_format: SerializationFormat) -> Self {
    Self {
      schema_filter: SchemaFilter::new(schema.clone()),
      mock_directive_extractor: MockDirectiveExtractor::new(schema.clone()),
      schema,
      serialization_format,
    }
  }

  pub(crate) fn split_operation(
    &self,
    query: &str,
    operation_name: Option<&str>,
    variables: &Map<ByteString, Value>,
    schema_extension: Option<&Value>,
  ) -> Result<SplitResult, SplitError> {
    match schema_extension {
      None => self.split_operation_inner(query, operation_name, variables),
      Some(Value::String(extended_sdl)) if !extended_sdl.as_str().is_empty() => self
        .with_extended_schema(extended_sdl.as_str())?
        .split_operation_inner(query, operation_name, variables),
      Some(_) => Err(SplitError::SchemaExtensionInput(
        "schemaExtension must be a non-empty string".to_string(),
      )),
    }
  }

  fn split_operation_inner(
    &self,
    operation: &str,
    operation_name: Option<&str>,
    variables: &Map<ByteString, Value>,
  ) -> Result<SplitResult, SplitError> {
    let scenario = self.mock_directive_extractor.extract(operation, operation_name)?;
    let (document, mocked_fields, is_partial) = match scenario {
      MockScenario::NoMocks => return Ok(SplitResult::NoMocks),
      MockScenario::Introspection(response) => return Ok(SplitResult::Introspection(response)),
      MockScenario::OperationMocked {
        document,
        mocked_fields,
        ..
      }
      | MockScenario::AllTopLevelFieldsMocked {
        document,
        mocked_fields,
      } => (document, mocked_fields, false),
      MockScenario::PartialMocked {
        document,
        mocked_fields,
      } => (document, mocked_fields, true),
    };

    let (to_mock_operation, upstream_operation) = if is_partial {
      let paths: Vec<String> = mocked_fields.iter().map(|field| field.path.clone()).collect();
      let operation_field_filter = OperationFieldFilter::new(&document, &paths);
      let filter = |mode: FilterMode| {
        operation_field_filter
          .filter(operation_name, mode)
          .map_err(|error| SplitError::OperationFilter(format!("{error:?}")))
      };

      let filtered_document = filter(FilterMode::Include)?;
      let excluded_document = filter(FilterMode::Exclude)?;
      (
        filtered_document.serialize().no_indent().to_string(),
        Some(excluded_document.serialize().no_indent().to_string()),
      )
    } else {
      (operation.to_string(), None)
    };

    let prompt = MockResponsePrompt::builder()
      .graphql_schema(self.schema_filter.filter(&to_mock_operation, operation_name)?.minify())
      .graphql_operation(to_mock_operation)
      .mocked_fields(mocked_fields)
      .variables(variables.clone())
      .format(self.serialization_format)
      .build();

    match upstream_operation {
      Some(upstream_operation) => Ok(SplitResult::PartialMock {
        prompt,
        upstream_operation,
      }),
      None => Ok(SplitResult::FullMock(prompt)),
    }
  }

  fn with_extended_schema(&self, extension_sdl: &str) -> Result<Self, ExtendSchemaError> {
    let mut builder = Schema::builder();
    for (index, (file_id, source)) in self.schema.sources.iter().enumerate() {
      if *file_id != FileId::BUILT_IN {
        builder = builder.parse(source.source_text(), format!("schema-{index}.graphql"));
      }
    }
    let schema = builder
      .parse(extension_sdl, "schema-extension.graphql")
      .build()
      .map_err(|error| ExtendSchemaError::Build(error.errors.iter().map(|d| d.error.to_string()).collect()))?
      .validate()
      .map_err(|error| ExtendSchemaError::Validate(error.errors.iter().map(|d| d.error.to_string()).collect()))?;

    Ok(Self::new(Arc::new(schema), self.serialization_format))
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use apollo_compiler::ExecutableDocument;

  fn test_planner(schema_sdl: &str) -> MockPlanner {
    let schema = Schema::builder()
      .parse(
        "directive @mock(hint: String) on QUERY | MUTATION | FIELD",
        "mock-directive.graphql",
      )
      .parse(schema_sdl, "schema.graphql")
      .build()
      .unwrap();
    let schema = Arc::new(schema.validate().unwrap());
    MockPlanner::new(schema, Default::default())
  }

  #[test]
  fn with_extended_schema_adds_field_to_schema() {
    let extended = test_planner("type Query { hello: String }")
      .with_extended_schema("extend type Query { newField: String }")
      .expect("should build extended schema");
    assert!(
      extended
        .schema
        .get_object("Query")
        .expect("Query type should exist")
        .fields
        .get("newField")
        .is_some(),
      "newField should exist in Query"
    );
  }

  #[test]
  fn with_extended_schema_returns_build_error_for_lexical_error() {
    let error = test_planner("type Query { hello: String }")
      .with_extended_schema("not valid graphql {{{")
      .expect_err("expected error for invalid SDL");
    assert!(matches!(error, ExtendSchemaError::Build(_)));
  }

  #[test]
  fn with_extended_schema_returns_validate_error_for_semantic_error() {
    let error = test_planner("type Query { hello: String }")
      .with_extended_schema("extend type Query { newField: NonExistentType }")
      .expect_err("expected validation error for undefined type");
    assert!(matches!(error, ExtendSchemaError::Validate(_)));
  }

  #[test]
  fn operation_with_extended_types_should_parse_against_extended_schema() {
    let base_schema = r"
      type Query { me: User }
      type User { id: ID name: String }
    ";
    let extensions = r"
      interface Address { street: String city: String }
      type HomeAddress implements Address { street: String city: String isResidential: Boolean }
      type WorkAddress implements Address { street: String city: String company: String }
      extend type User { address: Address }
    ";
    let extended = test_planner(base_schema)
      .with_extended_schema(extensions)
      .expect("should build extended schema");

    let operation = r"
      query {
        me {
          name
          address {
            street
            city
            ... on HomeAddress { isResidential }
            ... on WorkAddress { company }
          }
        }
      }
    ";
    assert!(ExecutableDocument::parse_and_validate(&extended.schema, operation, "operation.graphql").is_ok());
  }

  #[test]
  fn with_extended_schema_does_not_modify_original() {
    let planner = test_planner("type Query { hello: String }");
    let _extended = planner
      .with_extended_schema("extend type Query { newField: String }")
      .expect("should build extended schema");
    assert!(
      planner
        .schema
        .get_object("Query")
        .expect("Query type should exist")
        .fields
        .get("newField")
        .is_none(),
      "original planner should not have the extended field"
    );
  }

  #[test]
  fn split_operation_with_schema_extension_extends_schema() {
    let ext = Some(Value::String("extend type Query { newField: String }".into()));
    let result = test_planner("type Query { hello: String }")
      .split_operation("{ newField @mock }", None, &Map::new(), ext.as_ref())
      .expect("should succeed with extended schema");
    assert!(matches!(result, SplitResult::FullMock(_)));
  }

  #[test]
  fn split_operation_without_schema_extension_uses_original_schema() {
    let result = test_planner("type Query { hello: String }")
      .split_operation("{ hello @mock }", None, &Map::new(), None)
      .expect("should succeed with original schema");
    assert!(matches!(result, SplitResult::FullMock(_)));
  }

  #[test]
  fn split_operation_with_null_schema_extension_returns_error() {
    let ext = Some(Value::Null);
    let error = test_planner("type Query { hello: String }")
      .split_operation("{ hello @mock }", None, &Map::new(), ext.as_ref())
      .expect_err("should fail for null extension");
    assert!(matches!(error, SplitError::SchemaExtensionInput(_)));
  }

  #[test]
  fn split_operation_with_empty_schema_extension_returns_error() {
    let planner = test_planner("type Query { hello: String }");
    let ext = Some(Value::String("".into()));
    let error = planner
      .split_operation("{ hello @mock }", None, &Map::new(), ext.as_ref())
      .expect_err("should fail for empty extension");
    assert!(matches!(error, SplitError::SchemaExtensionInput(_)));
  }

  #[test]
  fn split_operation_with_non_string_schema_extension_returns_error() {
    let planner = test_planner("type Query { hello: String }");
    let ext = Some(serde_json_bytes::json!(42));
    let error = planner
      .split_operation("{ hello @mock }", None, &Map::new(), ext.as_ref())
      .expect_err("should fail for non-string");
    assert!(matches!(error, SplitError::SchemaExtensionInput(_)));
  }

  #[test]
  fn split_operation_with_invalid_sdl_returns_schema_extension_error() {
    let planner = test_planner("type Query { hello: String }");
    let ext = Some(Value::String("not valid graphql {{{".into()));
    let error = planner
      .split_operation("{ hello @mock }", None, &Map::new(), ext.as_ref())
      .expect_err("should fail for invalid SDL");
    assert!(matches!(error, SplitError::SchemaExtension(_)));
  }
}
