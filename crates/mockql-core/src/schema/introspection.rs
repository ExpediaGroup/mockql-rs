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

//! Schema loading via GraphQL introspection.

use crate::Header;
use cynic::http::CynicReqwestError;
use cynic::http::ReqwestExt;
use cynic_introspection::CapabilitySet;
use cynic_introspection::IntrospectionQuery;
use cynic_introspection::SchemaError;
use cynic_introspection::SpecificationVersion;
use reqwest::Client;
use reqwest::StatusCode;
use reqwest::Url;
use reqwest::header::HeaderMap;

/// Loads a validated Apollo schema by executing the standard introspection query.
pub async fn load_schema_with_introspection(
  url: &Url,
  headers: &[Header],
  client: &Client,
) -> Result<String, IntrospectionError> {
  let introspection_result =
    match execute_instrospection_query(url, headers, client, SpecificationVersion::October2021.capabilities()).await {
      Ok(introspection_result) => introspection_result,
      Err(IntrospectionError::GraphqlErrors(_)) => {
        // If the server returned GraphQL execution errors, try again with the June 2018 query.
        execute_instrospection_query(url, headers, client, SpecificationVersion::June2018.capabilities()).await?
      }
      Err(error) => return Err(error),
    };

  Ok(
    introspection_result
      .into_schema()
      .map_err(IntrospectionError::from)?
      .to_sdl(),
  )
}

pub async fn execute_instrospection_query(
  url: &Url,
  headers: &[Header],
  client: &Client,
  capability_set: CapabilitySet,
) -> Result<IntrospectionQuery, IntrospectionError> {
  let response = client
    .post(url.clone())
    .headers(headers.iter().cloned().collect::<HeaderMap>())
    .run_graphql(IntrospectionQuery::with_capabilities(capability_set))
    .await
    .map_err(IntrospectionError::from)?;

  if let Some(errors) = response.errors {
    return Err(IntrospectionError::GraphqlErrors(format!("{errors:?}")));
  }

  response.data.ok_or(IntrospectionError::MissingSchema)
}

/// Errors raised while resolving a schema through GraphQL introspection.
#[derive(Debug, thiserror::Error)]
pub enum IntrospectionError {
  /// The HTTP request failed before a response could be received.
  #[error("introspection request failed: {0}")]
  Request(#[source] reqwest::Error),
  /// The server returned a non-success HTTP status.
  #[error("introspection request returned HTTP status: {status}, body: {body}")]
  HttpStatus {
    /// HTTP status code returned by the introspection endpoint.
    status: StatusCode,
    /// Response body returned alongside the HTTP status.
    body: String,
  },
  /// The server returned JSON that could not be decoded.
  #[error("introspection response was not valid JSON: {0}")]
  Json(#[source] serde_json::Error),
  /// The introspection payload did not contain `data.__schema`.
  #[error("introspection response did not contain a schema")]
  MissingSchema,
  /// The introspection response contained GraphQL execution errors.
  #[error("introspection response contained GraphQL errors: {0}")]
  GraphqlErrors(String),
  /// The introspection response could not be converted into schema SDL.
  #[error("failed to convert introspection response: {0}")]
  Conversion(String),
  /// Apollo rejected the reconstructed schema document.
  #[error("failed to build Apollo schema: {0}")]
  Schema(String),
}

impl From<CynicReqwestError> for IntrospectionError {
  fn from(error: CynicReqwestError) -> Self {
    match error {
      CynicReqwestError::ReqwestError(error) => Self::Request(error),
      CynicReqwestError::ErrorResponse(status, body) => Self::HttpStatus { status, body },
    }
  }
}

impl From<SchemaError> for IntrospectionError {
  fn from(error: SchemaError) -> Self {
    match error {
      SchemaError::IntrospectionQueryFailed => Self::MissingSchema,
      _ => Self::Conversion(error.to_string()),
    }
  }
}
