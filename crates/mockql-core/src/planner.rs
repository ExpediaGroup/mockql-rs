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
use crate::graphql::abstract_typename_injector::add_typename_to_abstract_types;
use crate::graphql::mock_directive_extractor::MockDirectiveExtractor;
use crate::graphql::mock_directive_extractor::MockDirectiveExtractorError;
use crate::graphql::mock_directive_extractor::MockScenario;
use crate::graphql::mock_response_prompt::MockResponsePrompt;
use crate::graphql::mock_response_prompt::SerializationFormat;
use crate::graphql::operation_field_filter::FilterMode;
use crate::graphql::operation_field_filter::OperationFieldFilter;
use crate::graphql::response_validator::ResponseValidator;
use crate::graphql::schema_filter::SchemaFilter;
use crate::graphql::schema_filter::SchemaFilterError;
use apollo_compiler::ExecutableDocument;
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
  response_validator: ResponseValidator,
  schema_filter: SchemaFilter,
  mock_directive_extractor: MockDirectiveExtractor,
  serialization_format: SerializationFormat,
}

/// Internal result of splitting an operation into passthrough and mocked portions.
#[derive(Debug)]
pub(crate) enum SplitResult {
  /// The operation contains no mocks and should be forwarded upstream as-is.
  NoMocks,
  /// The operation is a pure introspection query answered from the local schema.
  Introspection(ExecutionResponse),
  /// The full operation should be satisfied by a mock provider.
  FullMock {
    /// Prompt describing the response to generate.
    prompt: MockResponsePrompt,
    /// Schema-bound validator for the generated response.
    validator: ResponseValidator,
    /// Operation document.
    document: Valid<ExecutableDocument>,
  },
  /// The operation should be split between upstream execution and provider generation.
  PartialMock {
    /// Prompt describing the portion that should be mocked.
    prompt: MockResponsePrompt,
    /// Schema-bound validator for the generated response.
    validator: ResponseValidator,
    /// Operation document.
    document: Valid<ExecutableDocument>,
    /// Original operation document used to validate the merged response.
    original_document: Box<Valid<ExecutableDocument>>,
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
  /// The prompt operation could not be selected for abstract type augmentation.
  #[error("failed to inject abstract type names: {0}")]
  AbstractTypenameInjection(String),
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
      response_validator: ResponseValidator::new(schema.clone()),
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
    let original_document = document.clone();

    let (to_mock_document, upstream_operation) = if is_partial {
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
        filtered_document,
        Some(excluded_document.serialize().no_indent().to_string()),
      )
    } else {
      (document, None)
    };
    let to_mock_operation = add_typename_to_abstract_types(&self.schema, to_mock_document.clone(), operation_name)
      .map_err(|error| SplitError::AbstractTypenameInjection(format!("{error:?}")))?
      .serialize()
      .no_indent()
      .to_string();

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
        validator: self.response_validator.clone(),
        document: to_mock_document,
        original_document: Box::new(original_document),
        upstream_operation,
      }),
      None => Ok(SplitResult::FullMock {
        prompt,
        validator: self.response_validator.clone(),
        document: to_mock_document,
      }),
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
