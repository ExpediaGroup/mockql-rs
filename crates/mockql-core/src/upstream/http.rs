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

//! HTTP helpers for executing upstream GraphQL requests.

use crate::Header;
use crate::graphql::GraphQLResponse;
use reqwest::StatusCode;
use reqwest::Url;
use reqwest::header::HeaderMap;
use serde_json::json;
use serde_json_bytes::ByteString;
use serde_json_bytes::Map;
use serde_json_bytes::Value;
use thiserror::Error;

/// Errors raised while building or executing upstream GraphQL requests.
#[derive(Debug, Error)]
pub enum UpstreamError {
  /// The upstream HTTP request failed before a response could be received.
  #[error("upstream request failed: {0}")]
  Request(#[source] reqwest::Error),
  /// The upstream server returned a non-success HTTP status.
  #[error("upstream request returned HTTP status: {status}, body: {body}")]
  HttpStatus {
    /// HTTP status code returned by the upstream server.
    status: StatusCode,
    /// Response body returned alongside the HTTP status.
    body: String,
  },
  /// The upstream response body was not valid GraphQL JSON.
  #[error("upstream returned invalid JSON: {0}")]
  InvalidJson(#[source] serde_json::Error),
}

/// Executes a GraphQL operation against an upstream endpoint.
pub async fn execute_graphql(
  client: &reqwest::Client,
  url: Url,
  headers: &[Header],
  operation: &str,
  operation_name: Option<&str>,
  variables: &Map<ByteString, Value>,
) -> Result<GraphQLResponse, UpstreamError> {
  let response = client
    .post(url)
    .headers(headers.iter().cloned().collect::<HeaderMap>())
    .json(&json!({
      "query": operation,
      "operationName": operation_name,
      "variables": variables,
    }))
    .send()
    .await
    .map_err(UpstreamError::Request)?;

  let status = response.status();
  if !status.is_success() {
    let body = response.text().await.unwrap_or_default();
    return Err(UpstreamError::HttpStatus { status, body });
  }

  let body: serde_json::Value = response.json().await.map_err(UpstreamError::Request)?;
  GraphQLResponse::try_from(body).map_err(UpstreamError::InvalidJson)
}
