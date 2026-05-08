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

//! Schema loading helpers for file-backed and introspected schemas.

mod introspection;

use crate::Header;
use apollo_compiler::Schema;
use apollo_compiler::ast::Definition;
use apollo_compiler::ast::Document;
use apollo_compiler::validation::Valid;
use apollo_compiler::validation::WithErrors;
pub use introspection::IntrospectionError;
pub use introspection::load_schema_with_introspection;
use reqwest::Client;
use reqwest::Url;
use std::path::Path;
use std::sync::LazyLock;

static MOCK_DIRECTIVE_DEFINITION: LazyLock<Document> = LazyLock::new(|| {
  Document::parse(
    "directive @mock(hint: String) on QUERY | MUTATION | FIELD",
    "mock-directive.graphql",
  )
  .expect("built-in @mock directive definition should parse")
});

/// Errors raised while resolving the active GraphQL schema.
#[derive(Debug, thiserror::Error)]
pub enum SchemaLoadError {
  /// Reading a local schema file failed.
  #[error("failed to read schema file: {0}")]
  Io(#[from] std::io::Error),
  /// The schema could not be parsed.
  #[error("failed to parse schema: {0:?}")]
  Parse(Vec<String>),
  /// The schema could not be validated.
  #[error("failed to validate schema: {0:?}")]
  Validate(Vec<String>),
  /// No GraphQL URL was provided for introspection-based schema loading.
  #[error("missing GraphQL URL for schema introspection")]
  MissingGraphQLUrl,
  /// Introspection-based schema loading failed.
  #[error(transparent)]
  Introspection(#[from] IntrospectionError),
}

/// Loads a validated schema from a local SDL file or from introspection.
pub async fn load_schema(
  schema_path: Option<&Path>,
  graphql_url: Option<&Url>,
  graphql_headers: &[Header],
  client: &Client,
) -> Result<Valid<Schema>, SchemaLoadError> {
  let sdl = match schema_path {
    Some(path) => std::fs::read_to_string(path)?,
    None => {
      load_schema_with_introspection(
        graphql_url.ok_or(SchemaLoadError::MissingGraphQLUrl)?,
        graphql_headers,
        client,
      )
      .await?
    }
  };
  to_schema_with_mock_directive(&sdl)
}

/// Parses and validates schema SDL, injecting the built-in `@mock` directive when absent.
pub fn to_schema_with_mock_directive(sdl: &str) -> Result<Valid<Schema>, SchemaLoadError> {
  let document = Document::parse(sdl, "schema.graphql").map_err(|e| SchemaLoadError::Parse(collect_errors(e)))?;

  let builder = Schema::builder().add_ast(&document);
  let builder = if document
    .definitions
    .iter()
    .any(|definition| matches!(definition, Definition::DirectiveDefinition(d) if d.name.as_str() == "mock"))
  {
    builder
  } else {
    builder.add_ast(&MOCK_DIRECTIVE_DEFINITION)
  };

  builder
    .build()
    .map_err(|e| SchemaLoadError::Parse(collect_errors(e)))?
    .validate()
    .map_err(|e| SchemaLoadError::Validate(collect_errors(e)))
}

fn collect_errors<T>(errors: WithErrors<T>) -> Vec<String> {
  errors.errors.iter().map(|d| d.error.to_string()).collect()
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn instrument_schema_adds_mock_directive_when_missing() {
    let schema = to_schema_with_mock_directive("type Query { hello: String }").expect("schema should instrument");
    let directive = schema
      .directive_definitions
      .get("mock")
      .expect("@mock directive should be present");

    assert_eq!(
      directive.serialize().to_string(),
      "directive @mock(hint: String) on QUERY | MUTATION | FIELD"
    );
  }

  #[test]
  fn instrument_schema_leaves_existing_mock_directive_unchanged() {
    let schema = to_schema_with_mock_directive(
      r"
        directive @mock(hint: String) on QUERY | MUTATION | FIELD
        type Query { hello: String }
      ",
    )
    .expect("schema should instrument");
    assert_eq!(
      schema
        .directive_definitions
        .keys()
        .filter(|name| name.as_str() == "mock")
        .count(),
      1
    );
  }
}
