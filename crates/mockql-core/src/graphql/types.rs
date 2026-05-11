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

//! Core GraphQL request and response types.

use super::minify::MinifyExt;
use super::response_merger::MergeableResponse;
use apollo_compiler::ast::NamedType;
use apollo_compiler::collections::IndexMap;
use apollo_compiler::schema::ExtendedType;
use serde::Deserialize;
use serde::Serialize;
use serde_json_bytes::ByteString;
use serde_json_bytes::Map;
use serde_json_bytes::Value;
use std::fmt;
use std::fmt::Display;

/// A standard GraphQL JSON request.
#[derive(Debug, Clone, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct GraphQLRequest {
  /// The GraphQL query string.
  pub query: String,
  /// The operation name to execute.
  pub operation_name: Option<String>,
  /// Optional variables for the operation.
  #[serde(default, deserialize_with = "deserialize_null_to_default")]
  pub variables: Map<ByteString, Value>,
  /// Optional extensions.
  #[serde(default, deserialize_with = "deserialize_null_to_default")]
  pub extensions: Map<ByteString, Value>,
}

/// The set of filtered types required to describe an operation.
#[derive(Debug)]
pub struct SchemaTypes(pub(crate) IndexMap<NamedType, ExtendedType>);

impl SchemaTypes {
  /// Create a new SchemaTypes
  pub fn new(types: IndexMap<NamedType, ExtendedType>) -> Self {
    Self(types)
  }
}

impl MinifyExt for SchemaTypes {
  fn minify(&self) -> String {
    self
      .0
      .iter()
      .map(|(_, type_)| format!("{}: {}", type_.name().as_str(), type_.minify()))
      .collect::<Vec<String>>()
      .join("\n")
  }
}

impl Display for SchemaTypes {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    for (_named_type, extended_type) in &self.0 {
      Display::fmt(&extended_type.serialize(), f)?;
    }
    Ok(())
  }
}

/// This is needed because GraphQL clients commonly send `"variables": null`
/// or `"extensions": null`, which would otherwise fail to deserialize
fn deserialize_null_to_default<'de, D, T>(deserializer: D) -> Result<T, D::Error>
where
  D: serde::Deserializer<'de>,
  T: Default + Deserialize<'de>,
{
  <Option<T>>::deserialize(deserializer).map(|x| x.unwrap_or_default())
}

/// A standard GraphQL JSON response.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct GraphQLResponse {
  /// The GraphQL `data` payload, if present.
  #[serde(skip_serializing_if = "Option::is_none")]
  pub data: Option<Value>,
  /// The GraphQL `errors` array.
  #[serde(default, skip_serializing_if = "Vec::is_empty")]
  pub errors: Vec<Value>,
  /// The GraphQL `extensions` object.
  #[serde(default, skip_serializing_if = "Map::is_empty")]
  pub extensions: Map<ByteString, Value>,
}

impl TryFrom<serde_json::Value> for GraphQLResponse {
  type Error = serde_json::Error;

  fn try_from(value: serde_json::Value) -> Result<Self, Self::Error> {
    serde_json::from_value(value)
  }
}

impl TryFrom<&str> for GraphQLResponse {
  type Error = serde_json::Error;

  fn try_from(value: &str) -> Result<Self, Self::Error> {
    serde_json::from_str(value)
  }
}

impl MergeableResponse for GraphQLResponse {
  fn data_mut(&mut self) -> &mut Option<Value> {
    &mut self.data
  }

  fn extensions_mut(&mut self) -> &mut Map<ByteString, Value> {
    &mut self.extensions
  }

  fn take_errors_from(&mut self, source: &mut Self) {
    self.errors.append(&mut source.errors);
  }
}
