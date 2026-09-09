// Copyright 2026 Expedia, Inc.
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

//! Validation and projection of generated GraphQL response data.

use apollo_compiler::ExecutableDocument;
use apollo_compiler::Schema;
use apollo_compiler::ast::Type;
use apollo_compiler::resolvers::Execution;
use apollo_compiler::resolvers::FieldError;
use apollo_compiler::resolvers::ObjectValue;
use apollo_compiler::resolvers::ResolveInfo;
use apollo_compiler::resolvers::ResolvedValue;
use apollo_compiler::response::GraphQLError;
use apollo_compiler::response::JsonMap;
use apollo_compiler::response::JsonValue;
use apollo_compiler::validation::Valid;
use serde_json_bytes::ByteString;
use serde_json_bytes::Map;
use serde_json_bytes::Value;
use std::sync::Arc;

/// Validates generated response data against a GraphQL schema and projects away
/// fields that an operation did not select.
#[derive(Debug, Clone)]
pub(crate) struct ResponseValidator {
  schema: Arc<Valid<Schema>>,
}

/// Errors raised while preparing or validating generated response data.
#[derive(Debug, thiserror::Error)]
pub enum ResponseValidationError {
  /// The requested operation could not be selected or its variables were invalid.
  #[error("invalid GraphQL request: {0}")]
  InvalidRequest(String),
  /// Generated response data was not a JSON object.
  #[error("response data must be a JSON object")]
  InvalidData,
  /// Apollo execution found values that do not satisfy the GraphQL schema.
  #[error("response validation failed: {0:?}")]
  Execution(Vec<GraphQLError>),
}

impl ResponseValidator {
  /// Creates a reusable validator for a GraphQL schema.
  pub(crate) fn new(schema: Arc<Valid<Schema>>) -> Self {
    Self { schema }
  }

  /// Validates response data and returns only fields selected by the operation.
  pub(crate) fn validate(
    &self,
    document: &Valid<ExecutableDocument>,
    operation_name: Option<&str>,
    variables: &Map<ByteString, Value>,
    data: &Value,
  ) -> Result<Value, ResponseValidationError> {
    let root_object = data.as_object().ok_or(ResponseValidationError::InvalidData)?;
    let operation = document
      .operations
      .get(operation_name)
      .map_err(|error| ResponseValidationError::InvalidRequest(error.message().to_string()))?;

    let response = Execution::new(&self.schema, document)
      .operation(operation)
      .raw_variable_values(variables)
      .execute_sync(&JsonObject {
        type_name: operation.selection_set.ty.as_str(),
        object: root_object,
      })
      .map_err(|error| ResponseValidationError::InvalidRequest(error.message().to_string()))?;

    if !response.errors.is_empty() {
      return Err(ResponseValidationError::Execution(response.errors));
    }

    response
      .data
      .map(Value::Object)
      .ok_or(ResponseValidationError::InvalidData)
  }
}

struct JsonObject<'a> {
  type_name: &'a str,
  object: &'a JsonMap,
}

impl ObjectValue for JsonObject<'_> {
  fn type_name(&self) -> &str {
    self.type_name
  }

  fn resolve_field<'a>(&'a self, info: &'a ResolveInfo<'a>) -> Result<ResolvedValue<'a>, FieldError> {
    let response_key = info.field_selections()[0].response_key();
    match self.object.get(response_key.as_str()) {
      Some(value) => resolve_value(value, info),
      None => Ok(ResolvedValue::null()),
    }
  }
}

fn resolve_value<'a>(value: &'a JsonValue, info: &'a ResolveInfo<'a>) -> Result<ResolvedValue<'a>, FieldError> {
  resolve_typed_value(value, &info.field_definition().ty, info)
}

fn resolve_typed_value<'a>(
  value: &'a JsonValue,
  ty: &'a Type,
  info: &'a ResolveInfo<'a>,
) -> Result<ResolvedValue<'a>, FieldError> {
  if ty.is_list() {
    return match value {
      JsonValue::Array(values) => Ok(ResolvedValue::List(Box::new(
        values
          .iter()
          .map(move |value| resolve_typed_value(value, ty.item_type(), info)),
      ))),
      value => Ok(ResolvedValue::leaf(value.clone())),
    };
  }

  let type_name = ty.inner_named_type();
  let type_definition = &info.schema().types[type_name];
  if type_definition.is_scalar() || type_definition.is_enum() {
    return Ok(ResolvedValue::leaf(normalize_leaf(value, type_name.as_str())));
  }

  match value {
    JsonValue::Object(object) => Ok(ResolvedValue::object(JsonObject {
      type_name: object
        .get("__typename")
        .and_then(JsonValue::as_str)
        .unwrap_or(type_name.as_str()),
      object,
    })),
    value => Ok(ResolvedValue::leaf(value.clone())),
  }
}

fn normalize_leaf(value: &JsonValue, type_name: &str) -> JsonValue {
  match type_name {
    "Float" if !value.is_f64() => value
      .as_i64()
      .and_then(|integer| {
        let float = integer as f64;
        (integer != i64::MAX && float as i64 == integer).then_some(float)
      })
      .or_else(|| {
        value.as_u64().and_then(|integer| {
          let float = integer as f64;
          (integer != u64::MAX && float as u64 == integer).then_some(float)
        })
      })
      .map_or_else(|| value.clone(), JsonValue::from),
    "ID" => value
      .as_i64()
      .map_or_else(|| value.clone(), |integer| JsonValue::from(integer.to_string())),
    _ => value.clone(),
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use serde_json::Value as JsonValue;
  use serde_json::from_str;
  use std::fs::read_to_string;

  fn parse_fixture(content: &str) -> (String, Value) {
    let mut fixture: serde_json::Map<String, JsonValue> = from_str(content).expect("should parse fixture JSON");
    (
      serde_json::from_value(
        fixture
          .remove("operation")
          .expect("fixture should have 'operation' key"),
      )
      .expect("should deserialize 'operation' string"),
      serde_json::from_value(fixture.remove("data").expect("fixture should have 'data' key"))
        .expect("should deserialize response data"),
    )
  }

  fn snapshot(result: Result<Value, ResponseValidationError>) -> String {
    let value = match result {
      Ok(data) => serde_json::json!({ "data": data }),
      Err(ResponseValidationError::Execution(errors)) => serde_json::json!({ "errors": errors }),
      Err(error) => serde_json::json!({ "error": error.to_string() }),
    };
    serde_json::to_string_pretty(&value).expect("snapshot value should serialize")
  }

  #[test]
  fn validate_response() {
    insta::glob!("snapshots/response_validator/*.json", |path| {
      let (operation, data) = parse_fixture(&read_to_string(path).expect("should read fixture file"));
      let schema = Schema::parse_and_validate(
        concat!(
          include_str!("schema.graphql"),
          "scalar JSON extend type Query { config: JSON!, configs: [JSON!]! }"
        ),
        "schema.graphql",
      )
      .expect("test schema should be valid");
      let document = ExecutableDocument::parse_and_validate(&schema, &operation, "operation.graphql")
        .expect("test operation should be valid");
      let result = ResponseValidator::new(Arc::new(schema)).validate(&document, None, &Map::new(), &data);

      insta::with_settings!({snapshot_path => path.parent().unwrap(), omit_expression => true}, {
        insta::assert_snapshot!(snapshot(result));
      });
    });
  }
}
