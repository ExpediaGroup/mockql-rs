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

//! Response-merging helpers for partial mock execution.

use serde_json_bytes::ByteString;
use serde_json_bytes::Map;
use serde_json_bytes::Value;

/// Provides mutable access to the three GraphQL response fields needed by
/// [`ResponseMerger`]: `data`, `errors`, and `extensions`.
pub trait MergeableResponse {
  /// Returns mutable access to the response `data` payload.
  fn data_mut(&mut self) -> &mut Option<Value>;
  /// Returns mutable access to the response `extensions` object.
  fn extensions_mut(&mut self) -> &mut Map<ByteString, Value>;
  /// Moves any accumulated GraphQL errors from `source` into `self`.
  fn take_errors_from(&mut self, source: &mut Self);
}

/// Errors raised while merging partial GraphQL responses.
#[derive(Debug, thiserror::Error)]
pub enum ResponseMergerError {
  /// Two scalar or otherwise non-mergeable values collided at the given path.
  #[error("non-mergeable keys at path: {path}: {details}")]
  NonMergeableKeys {
    /// Dot-separated response path where the collision occurred.
    path: String,
    /// Human-readable explanation of why the values cannot be merged.
    details: String,
  },
  /// Arrays at the same response path had different lengths and could not be merged.
  #[error("non-mergeable arrays because their lengths differ at path: {path} (into: {into}, from: {from})")]
  MismatchedArrayLengths {
    /// Dot-separated response path where the mismatch occurred.
    path: String,
    /// Number of items present in the destination response.
    into: usize,
    /// Number of items present in the source response.
    from: usize,
  },
  /// A response `data` value was neither an object nor `null`.
  #[error("found invalid type in '{0}' response, expected data to be an object or null")]
  InvalidDataValueType(String),
}

fn type_name(value: &Value) -> &'static str {
  match value {
    Value::Null => "null",
    Value::Bool(_) => "bool",
    Value::Number(_) => "number",
    Value::String(_) => "string",
    Value::Array(_) => "array",
    Value::Object(_) => "object",
  }
}

/// Merges two [`MergeableResponse`] objects that have disjoint non-object keys.
#[derive(Debug)]
pub struct ResponseMerger;

impl ResponseMerger {
  /// Merges `from` into `into`, combining `data`, `errors`, and non-conflicting `extensions`.
  pub fn merge<T: MergeableResponse>(into: &mut T, from: &mut T) -> Result<(), ResponseMergerError> {
    if matches!(into.data_mut(), Some(value) if !value.is_null() && !value.is_object()) {
      return Err(ResponseMergerError::InvalidDataValueType("into".into()));
    }
    if matches!(from.data_mut(), Some(value) if !value.is_null() && !value.is_object()) {
      return Err(ResponseMergerError::InvalidDataValueType("from".into()));
    }

    let into_data = into.data_mut().take();
    let from_data = from.data_mut().take();

    *into.data_mut() = match (into_data, from_data) {
      (Some(Value::Object(mut into_map)), Some(Value::Object(from_map))) => {
        Self::merge_objects(&mut into_map, from_map, "")?;
        Some(Value::Object(into_map))
      }
      (Some(data), _) | (_, Some(data)) if data.is_object() => Some(data),
      _ => None,
    };

    into.take_errors_from(from);

    let from_extensions = std::mem::take(from.extensions_mut());
    for (key, value) in from_extensions {
      let ext = into.extensions_mut();
      if !ext.contains_key(&key) {
        ext.insert(key, value);
      }
    }

    Ok(())
  }

  fn merge_objects(
    into: &mut Map<ByteString, Value>,
    from: Map<ByteString, Value>,
    path: &str,
  ) -> Result<(), ResponseMergerError> {
    for (from_key, from_value) in from {
      match (into.get_mut(&from_key), from_value) {
        // Key only in `from`
        (None, from_value) => {
          into.insert(from_key, from_value);
        }
        // Key exists on both sides, so recurse into the shared value.
        (Some(into_value), from_value) => {
          Self::merge_values(into_value, from_value, &Self::child_path(path, from_key.as_str()))?;
        }
      }
    }
    Ok(())
  }

  fn merge_values(into: &mut Value, from: Value, path: &str) -> Result<(), ResponseMergerError> {
    match (into, from) {
      (Value::Object(into_map), Value::Object(from_map)) => Self::merge_objects(into_map, from_map, path),
      (Value::Null, from @ (Value::Object(_) | Value::Array(_))) => Err(ResponseMergerError::NonMergeableKeys {
        path: path.into(),
        details: format!(
          "Upstream GraphQL response returned null for this field, cannot merge mocked {} into null",
          type_name(&from)
        ),
      }),
      (Value::Array(into_array), Value::Array(from_array)) => Self::merge_arrays(into_array, from_array, path),
      (lhs, rhs) => Err(ResponseMergerError::NonMergeableKeys {
        path: path.into(),
        details: format!(
          "values cannot be merged (into: {}, from: {})",
          type_name(lhs),
          type_name(&rhs)
        ),
      }),
    }
  }

  fn merge_arrays(into: &mut [Value], from: Vec<Value>, path: &str) -> Result<(), ResponseMergerError> {
    if into.len() != from.len() {
      return Err(ResponseMergerError::MismatchedArrayLengths {
        path: path.into(),
        into: into.len(),
        from: from.len(),
      });
    }

    for (index, (into_value, from_value)) in into.iter_mut().zip(from).enumerate() {
      let index = index.to_string();
      Self::merge_values(into_value, from_value, &Self::child_path(path, &index))?;
    }

    Ok(())
  }

  fn child_path(path: &str, key: &str) -> String {
    if path.is_empty() {
      key.into()
    } else {
      format!("{path}.{key}")
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::graphql::GraphQLResponse;
  use serde_json::Value as JsonValue;
  use serde_json::from_str;
  use std::fs::read_to_string;

  fn parse_fixture(content: &str) -> (GraphQLResponse, GraphQLResponse) {
    let mut fixture: serde_json::Map<String, JsonValue> = from_str(content).expect("should parse fixture JSON");
    (
      serde_json::from_value(fixture.remove("into").expect("fixture should have 'into' key"))
        .expect("should deserialize 'into' GraphQLResponse"),
      serde_json::from_value(fixture.remove("from").expect("fixture should have 'from' key"))
        .expect("should deserialize 'from' GraphQLResponse"),
    )
  }

  #[test]
  fn response_merger_produces_expected_output() {
    insta::glob!("snapshots/response_merger/[!e]*/*.json", |path| {
      let (mut into, mut from) = parse_fixture(&read_to_string(path).expect("should read fixture file"));
      ResponseMerger::merge(&mut into, &mut from).expect("merge should succeed");

      insta::with_settings!({snapshot_path => path.parent().unwrap(), omit_expression => true}, {
        insta::assert_snapshot!(serde_json::to_string_pretty(&into).unwrap());
      });
    });
  }

  #[test]
  fn response_merger_produces_expected_error() {
    insta::glob!("snapshots/response_merger/error_cases/*.json", |path| {
      let (mut into, mut from) = parse_fixture(&read_to_string(path).expect("should read fixture file"));
      let err = ResponseMerger::merge(&mut into, &mut from).expect_err("merge should fail");

      insta::with_settings!({snapshot_path => path.parent().unwrap(), omit_expression => true}, {
        insta::assert_snapshot!(format!("{err:?}"));
      });
    });
  }
}
