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

//! Extraction and classification of `@mock` directives within GraphQL operations.

use apollo_compiler::ExecutableDocument;
use apollo_compiler::Schema;
use apollo_compiler::ast::FieldDefinition;
use apollo_compiler::ast::Value;
use apollo_compiler::executable::Directive;
use apollo_compiler::executable::Field;
use apollo_compiler::executable::FragmentSpread;
use apollo_compiler::executable::InlineFragment;
use apollo_compiler::executable::Selection;
use apollo_compiler::executable::SelectionSet;
use apollo_compiler::introspection::partial_execute;
use apollo_compiler::response::ExecutionResponse;
use apollo_compiler::response::JsonMap;
use apollo_compiler::schema::ExtendedType;
use apollo_compiler::validation::Valid;
use serde::Deserialize;
use serde::Serialize;
use std::collections::HashSet;
use std::sync::Arc;

const MOCK_DIRECTIVE_NAME: &str = "mock";
const MOCK_DIRECTIVE_HINT_ARGUMENT_NAME: &str = "hint";

/// Extracts `@mock` directives from a GraphQL operation and classifies the result
/// into a [`MockScenario`] describing how the operation should be mocked.
#[derive(Debug)]
pub struct MockDirectiveExtractor {
  schema: Arc<Valid<Schema>>,
}

/// The mock behavior selected for an analyzed GraphQL operation.
#[derive(Debug, Clone, PartialEq, Serialize, Default)]
#[serde(tag = "type", content = "data")]
pub enum MockScenario {
  /// The operation contains no `@mock` directives.
  #[default]
  NoMocks,
  /// The operation is a pure introspection query answered from the local schema.
  Introspection(ExecutionResponse),
  /// The operation itself is annotated with `@mock`.
  OperationMocked {
    /// the validated document
    #[serde(serialize_with = "serialize_document")]
    document: Valid<ExecutableDocument>,
    /// Optional hint supplied on the operation-level `@mock`.
    hint: Option<String>,
    /// Fields marked with mock.
    mocked_fields: Vec<MockedField>,
  },
  /// Every top-level field is individually mocked.
  AllTopLevelFieldsMocked {
    /// the validated document
    #[serde(serialize_with = "serialize_document")]
    document: Valid<ExecutableDocument>,
    /// Fields marked with mock.
    mocked_fields: Vec<MockedField>,
  },
  /// The operation mixes mocked and non mocked fields.
  PartialMocked {
    /// the validated document
    #[serde(serialize_with = "serialize_document")]
    document: Valid<ExecutableDocument>,
    /// the operation mocked fields
    mocked_fields: Vec<MockedField>,
  },
}

fn serialize_document<S>(document: &Valid<ExecutableDocument>, serializer: S) -> Result<S::Ok, S::Error>
where
  S: serde::Serializer,
{
  serializer.serialize_str(&document.serialize().no_indent().to_string())
}

/// Errors raised while extracting `@mock` directives from an operation.
#[derive(Debug, thiserror::Error)]
pub enum MockDirectiveExtractorError {
  /// The GraphQL operation could not be parsed against the schema.
  #[error("failed to parse operation: {0:?}")]
  ParseError(Vec<String>),
  /// The GraphQL operation could not be validated against the schema.
  #[error("failed to validate operation: {0:?}")]
  Validate(Vec<String>),
  /// The requested operation name was not present in the document.
  #[error("operation not found: {0}")]
  OperationNotFound(String),
  /// Invalid directive placement or argument values were encountered.
  #[error("traversal errors: {0:?}")]
  TraversalErrors(Vec<String>),
  /// Introspection execution against the local schema failed.
  #[error("introspection execution failed: {0}")]
  Introspection(String),
}

/// Describes a single field that should be mocked.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MockedField {
  /// Dot-separated response path for the field.
  pub path: String,
  /// Optional generation hint for the field.
  pub hint: Option<String>,
  /// Innermost named GraphQL return type for the field.
  pub return_type: String,
}

impl MockDirectiveExtractor {
  /// Creates a new extractor for the supplied validated schema.
  pub fn new(schema: Arc<Valid<Schema>>) -> Self {
    Self { schema }
  }

  /// Parses the given query and classifies its `@mock` usage.
  pub fn extract(
    &self,
    query: &str,
    operation_name: Option<&str>,
  ) -> Result<MockScenario, MockDirectiveExtractorError> {
    let document = ExecutableDocument::parse(&self.schema, query, "query.graphql")
      .map_err(|e| MockDirectiveExtractorError::ParseError(e.errors.iter().map(|d| d.error.to_string()).collect()))?;

    let valid_document = document
      .validate(&self.schema)
      .map_err(|e| MockDirectiveExtractorError::Validate(e.errors.iter().map(|d| d.error.to_string()).collect()))?;

    let operation = valid_document
      .operations
      .get(operation_name)
      .map_err(|e| MockDirectiveExtractorError::OperationNotFound(format!("{e:?}")))?;

    if operation.is_introspection(&valid_document) {
      let response = partial_execute(
        &self.schema,
        &self.schema.implementers_map(),
        &valid_document,
        operation,
        &Valid::assume_valid(JsonMap::default()),
      )
      .map_err(|e| MockDirectiveExtractorError::Introspection(format!("{e:?}")))?;
      return Ok(MockScenario::Introspection(response));
    }

    let maybe_operation_mock = operation
      .directives
      .iter()
      .find(|d| d.name.as_str() == MOCK_DIRECTIVE_NAME)
      .map(|directive| get_hint(directive));

    let (mocked_fields, all_top_level_fields_mocked, errors) = {
      let mut traversal = MockExtractionTraversal::new(&self.schema, &valid_document);
      traversal.traverse(
        &operation.selection_set,
        operation.selection_set.ty.as_str(),
        "",
        maybe_operation_mock.is_some(),
      );
      let all_top_level_fields_mocked =
        !traversal.top_level_fields.is_empty() && traversal.top_level_fields == traversal.mocked_top_level_fields;
      (traversal.mocked_fields, all_top_level_fields_mocked, traversal.errors)
    };

    if !errors.is_empty() {
      Err(MockDirectiveExtractorError::TraversalErrors(errors))
    } else if let Some(hint) = maybe_operation_mock {
      Ok(MockScenario::OperationMocked {
        document: valid_document,
        hint,
        mocked_fields,
      })
    } else if mocked_fields.is_empty() {
      Ok(MockScenario::NoMocks)
    } else if all_top_level_fields_mocked {
      Ok(MockScenario::AllTopLevelFieldsMocked {
        document: valid_document,
        mocked_fields,
      })
    } else {
      Ok(MockScenario::PartialMocked {
        document: valid_document,
        mocked_fields,
      })
    }
  }
}

struct MockExtractionTraversal<'a> {
  schema: &'a Valid<Schema>,
  document: &'a Valid<ExecutableDocument>,
  mocked_fields: Vec<MockedField>,
  errors: Vec<String>,
  top_level_fields: HashSet<String>,
  mocked_top_level_fields: HashSet<String>,
}

impl<'a> MockExtractionTraversal<'a> {
  fn new(schema: &'a Valid<Schema>, document: &'a Valid<ExecutableDocument>) -> Self {
    Self {
      schema,
      document,
      mocked_fields: Vec::new(),
      errors: Vec::new(),
      top_level_fields: HashSet::new(),
      mocked_top_level_fields: HashSet::new(),
    }
  }

  fn traverse(&mut self, selection_set: &SelectionSet, parent_type_name: &str, path: &str, has_mocked_ancestor: bool) {
    // If all direct field children are mocked, the parent would become an empty selection set after
    // removing mocked fields. This is only valid when some ancestor is also mocked.
    if !path.is_empty() && !has_mocked_ancestor {
      let mut fields = Vec::new();
      let mut has_non_field_selections = false;
      for selection in &selection_set.selections {
        match selection {
          Selection::Field(field) => fields.push(field),
          _ => has_non_field_selections = true,
        }
      }

      let all_mocked = !fields.is_empty()
        && !has_non_field_selections
        && fields.iter().all(|field| {
          field
            .directives
            .iter()
            .any(|directive| directive.name.as_str() == MOCK_DIRECTIVE_NAME)
        });
      if all_mocked {
        self.errors.push(format!(
          "All fields under '{path}' have @mock - this would leave an empty selection set after removal"
        ));
        return;
      }
    }

    for selection in &selection_set.selections {
      match selection {
        Selection::FragmentSpread(spread) => self.process_fragment_spread(spread, path, has_mocked_ancestor),
        Selection::InlineFragment(inline) => {
          self.process_inline_fragment(inline, parent_type_name, path, has_mocked_ancestor);
        }
        Selection::Field(field) => self.process_field(field, parent_type_name, path, has_mocked_ancestor),
      }
    }
  }

  fn process_fragment_spread(&mut self, spread: &FragmentSpread, path: &str, has_mocked_ancestor: bool) {
    if let Some(fragment) = self.document.fragments.get(&spread.fragment_name) {
      self.traverse(
        &fragment.selection_set,
        fragment.type_condition().as_ref(),
        path,
        has_mocked_ancestor,
      );
    }
  }

  fn process_inline_fragment(
    &mut self,
    inline: &InlineFragment,
    parent_type_name: &str,
    path: &str,
    has_mocked_ancestor: bool,
  ) {
    let type_name = inline
      .type_condition
      .as_ref()
      .map_or_else(|| parent_type_name.to_string(), ToString::to_string);
    self.traverse(&inline.selection_set, &type_name, path, has_mocked_ancestor);
  }

  fn process_field(&mut self, field: &Field, parent_type_name: &str, path: &str, has_mocked_ancestor: bool) {
    let field_name = field.name.to_string();
    let response_key = field.response_key().to_string();
    let field_path = if path.is_empty() {
      response_key.clone()
    } else {
      format!("{path}.{response_key}")
    };

    if path.is_empty() {
      self.top_level_fields.insert(response_key.clone());
    }

    if let Some(mock_directive) = field.directives.iter().find(|d| d.name.as_str() == MOCK_DIRECTIVE_NAME) {
      if field_name.starts_with("__") {
        self.errors.push(format!(
          "@mock directive is not allowed on introspection field '{field_name}' at '{field_path}'"
        ));
        return;
      }

      if path.is_empty() {
        self.mocked_top_level_fields.insert(response_key);
      }

      let field_def = self
        .get_field_definition(parent_type_name, &field_name)
        .expect("field definition must exist for a field within a @mock directive");

      let return_type = field_def.ty.inner_named_type().to_string();
      let hint = get_hint(mock_directive);

      self.traverse(&field.selection_set, &return_type, &field_path, true);
      self.mocked_fields.push(MockedField {
        path: field_path,
        hint,
        return_type,
      });
    } else if let Some(field_def) = self.get_field_definition(parent_type_name, &field_name) {
      let return_type_name = field_def.ty.inner_named_type().to_string();
      self.traverse(
        &field.selection_set,
        &return_type_name,
        &field_path,
        has_mocked_ancestor,
      );
    }
  }

  fn get_field_definition(&self, type_name: &str, field_name: &str) -> Option<&FieldDefinition> {
    match self.schema.types.get(type_name)? {
      ExtendedType::Object(object) => object.fields.get(field_name).map(AsRef::as_ref),
      ExtendedType::Interface(interface) => interface.fields.get(field_name).map(AsRef::as_ref),
      _ => None,
    }
  }
}

fn get_hint(directive: &Directive) -> Option<String> {
  directive
    .arguments
    .iter()
    .find(|arg| arg.name.as_str() == MOCK_DIRECTIVE_HINT_ARGUMENT_NAME)
    .map(|arg| match &*arg.value {
      Value::String(s) => s.clone(),
      // SAFETY: document validation guarantees hint is a String
      _ => unreachable!("document validation guarantees hint argument is a String"),
    })
}

#[cfg(test)]
mod tests {
  use super::*;
  use std::fs::read_to_string;
  use std::sync::LazyLock;

  static MOCK_DIRECTIVE_EXTRACTOR: LazyLock<MockDirectiveExtractor> = LazyLock::new(|| {
    let sdl = include_str!("schema.graphql");
    let schema = Schema::parse_and_validate(sdl, "schema.graphql").expect("schema should be valid");
    MockDirectiveExtractor::new(Arc::new(schema))
  });

  #[test]
  fn mock_directive_extraction() {
    insta::glob!("snapshots/mock_directive_extractor/**/*.graphql", |path| {
      let content = read_to_string(path).expect("should read query file");
      let snapshot_path = path.parent().unwrap();
      let operation_name = content
        .lines()
        .next()
        .and_then(|line| line.strip_prefix("# operation_name:"))
        .map(|value| value.trim().to_string());
      insta::with_settings!({snapshot_path => snapshot_path}, {
        insta::assert_snapshot!(match MOCK_DIRECTIVE_EXTRACTOR.extract(&content, operation_name.as_deref()) {
          Ok(scenario) => serde_json::to_string_pretty(&scenario).unwrap(),
          Err(error) => format!("{error:?}"),
        });
      });
    });
  }
}
