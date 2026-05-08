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

//! Prompt structures and serialization helpers for mock providers.

use super::mock_directive_extractor::MockedField;
use schemars::JsonSchema;
use serde::Deserialize;
use serde::Serialize;
use serde_json_bytes::ByteString;
use serde_json_bytes::Map;
use serde_json_bytes::Value;

/// Extension key used to store mock-response metadata.
pub const MOCK_RESPONSE_EXTENSION_KEY: &str = "mockResponse";
/// Nested debug key stored inside [`MOCK_RESPONSE_EXTENSION_KEY`].
pub const DEBUG_KEY: &str = "debug";
/// Key used to store the rendered provider prompt.
pub const PROMPT_MARKDOWN_KEY: &str = "promptMarkdown";
/// Key used to store model reasoning for a generated response.
pub const REASONING_KEY: &str = "reasoning";
/// Key used to carry a schema extension payload in GraphQL extensions.
pub const SCHEMA_EXTENSION_KEY: &str = "schemaExtension";

const PROMPT_TEMPLATE: &str = include_str!("prompt-template.md");

/// Output format used when serializing structured prompt sections.
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum SerializationFormat {
  /// Serialize structured sections as JSON.
  #[default]
  Json,
  /// Serialize structured sections in TOON format.
  Toon,
}

/// Errors raised while serializing a [`MockResponsePrompt`].
#[derive(Debug, thiserror::Error)]
pub enum PromptSerializationError {
  /// TOON serialization failed.
  #[error("TOON serialization failed: {0}")]
  Toon(#[from] toon_format::ToonError),
  /// JSON serialization failed.
  #[error("JSON serialization failed: {0}")]
  Json(#[from] serde_json::Error),
}

/// Prompt payload sent to a mock provider.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MockResponsePrompt {
  graphql_schema: String,
  graphql_operation: String,
  variables: Map<ByteString, Value>,
  mocked_fields: Vec<MockedField>,
  #[serde(default)]
  partial_response: Option<Value>,
  #[serde(default)]
  format: SerializationFormat,
}

const DEBUG_EXCLUDED_KEYS: &[&str] = &["graphqlSchema", "variables"];

fn prompt_template() -> &'static str {
  PROMPT_TEMPLATE
    .strip_suffix("\r\n")
    .or_else(|| PROMPT_TEMPLATE.strip_suffix('\n'))
    .unwrap_or(PROMPT_TEMPLATE)
}

fn render_prompt_template(template: &str, replacements: &[(&str, &str)]) -> String {
  let mut rendered = String::with_capacity(template.len());
  let mut cursor = 0;

  while cursor < template.len() {
    let remaining = &template[cursor..];
    if let Some((token, value)) = replacements.iter().find(|(token, _)| remaining.starts_with(*token)) {
      rendered.push_str(value);
      cursor += token.len();
      continue;
    }

    let ch = remaining
      .chars()
      .next()
      .expect("remaining template should not be empty");
    rendered.push(ch);
    cursor += ch.len_utf8();
  }

  rendered
}

#[buildstructor::buildstructor]
impl MockResponsePrompt {
  #[builder(visibility = "pub(crate)")]
  fn new(
    graphql_schema: String,
    graphql_operation: String,
    variables: Map<ByteString, Value>,
    mocked_fields: Vec<MockedField>,
    format: SerializationFormat,
  ) -> Self {
    Self {
      graphql_schema,
      graphql_operation,
      variables,
      mocked_fields,
      partial_response: None,
      format,
    }
  }

  /// Returns a copy of the prompt augmented with upstream partial response data.
  #[must_use]
  pub fn with_partial_response(self, partial_response: Option<Value>) -> Self {
    Self {
      partial_response,
      ..self
    }
  }

  fn encode(&self, value: &impl serde::Serialize) -> Result<String, PromptSerializationError> {
    Ok(match self.format {
      SerializationFormat::Json => {
        let encoded = serde_json::to_string(value)?;
        format!("```json\n{encoded}\n```")
      }
      SerializationFormat::Toon => {
        let options = toon_format::EncodeOptions::new().with_key_folding(toon_format::types::KeyFoldingMode::Safe);
        let encoded = toon_format::encode(value, &options)?;
        format!("```toon\n{encoded}\n```")
      }
    })
  }

  /// Attaches prompt debug information to a GraphQL response `extensions` object.
  pub fn add_to_extensions(&self, extensions: &mut Map<ByteString, Value>) {
    let Ok(Value::Object(mut debug)) = serde_json_bytes::to_value(self) else {
      return;
    };
    for key in DEBUG_EXCLUDED_KEYS {
      debug.remove(*key);
    }
    if let Ok(markdown) = self.to_markdown() {
      debug.insert(PROMPT_MARKDOWN_KEY, Value::String(markdown.into()));
    }

    if let Some(Value::Object(mock_response_extension)) =
      extensions.get_mut(&ByteString::from(MOCK_RESPONSE_EXTENSION_KEY))
    {
      mock_response_extension.insert(DEBUG_KEY, Value::Object(debug));
    } else {
      let mut new_mock_response_extension = Map::new();
      new_mock_response_extension.insert(DEBUG_KEY, Value::Object(debug));
      extensions.insert(MOCK_RESPONSE_EXTENSION_KEY, Value::Object(new_mock_response_extension));
    }
  }

  /// Renders the prompt as provider-facing markdown instructions.
  pub fn to_markdown(&self) -> Result<String, PromptSerializationError> {
    let variables_block = self.encode(&self.variables)?;
    let mocked_fields_block = self.encode(&self.mocked_fields)?;

    let resolved_fields_section = match &self.partial_response {
      Some(data) => {
        let block = self.encode(data)?;
        format!(
          "### Partial Response\n\
          GraphQL response returned by the upstream GraphQL server. Use it for contextual mocks.\n\
          {block}"
        )
      }
      None => String::new(),
    };

    Ok(render_prompt_template(
      prompt_template(),
      &[
        ("__GRAPHQL_SCHEMA__", &self.graphql_schema),
        ("__GRAPHQL_OPERATION__", &self.graphql_operation),
        ("__VARIABLES_BLOCK__", &variables_block),
        ("__MOCKED_FIELDS_BLOCK__", &mocked_fields_block),
        ("__OPTIONAL_PARTIAL_RESPONSE_SECTION__", &resolved_fields_section),
        ("__MOCK_RESPONSE_EXT__", MOCK_RESPONSE_EXTENSION_KEY),
        ("__REASONING_EXT__", REASONING_KEY),
      ],
    ))
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use serde::Deserialize;
  use std::fs::read_to_string;

  #[derive(Debug, Deserialize)]
  struct PromptFixture {
    graphql_schema: String,
    graphql_operation: String,
    variables: Map<ByteString, Value>,
    mocked_fields: Vec<MockedField>,
    #[serde(default)]
    format: SerializationFormat,
    #[serde(default)]
    partial_response: Option<Value>,
  }

  #[test]
  fn to_markdown_matches_snapshot() {
    insta::glob!("snapshots/mock_response_prompt/*.json", |path| {
      let fixture: PromptFixture =
        serde_json::from_str(&read_to_string(path).expect("should read fixture file")).expect("should parse fixture");

      let prompt = MockResponsePrompt::builder()
        .graphql_schema(fixture.graphql_schema)
        .graphql_operation(fixture.graphql_operation)
        .variables(fixture.variables)
        .mocked_fields(fixture.mocked_fields)
        .format(fixture.format)
        .build()
        .with_partial_response(fixture.partial_response);

      let markdown = prompt.to_markdown().expect("markdown generation should succeed");

      insta::with_settings!({snapshot_path => path.parent().unwrap(), omit_expression => true}, {
        insta::assert_snapshot!(markdown);
      });
    });
  }

  #[test]
  fn add_to_extensions_inserts_into_existing_mock_response() {
    let prompt = MockResponsePrompt::builder()
      .graphql_schema("T:Query:me:User".to_string())
      .graphql_operation("{ me { name } }".to_string())
      .variables(Map::<ByteString, Value>::new())
      .mocked_fields(vec![])
      .format(SerializationFormat::Json)
      .build();

    let mut extensions = Map::new();
    let mut mock_response = Map::new();
    mock_response.insert("reasoning", Value::String("test reasoning".into()));
    extensions.insert(MOCK_RESPONSE_EXTENSION_KEY, Value::Object(mock_response));

    prompt.add_to_extensions(&mut extensions);

    insta::with_settings!({snapshot_path => "snapshots/mock_response_prompt"}, {
      insta::assert_snapshot!(serde_json::to_string_pretty(&extensions).unwrap());
    });
  }
}
