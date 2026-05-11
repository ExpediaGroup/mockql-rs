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

use crate::commands::proxy::config::SharedProxyConfig;
use crate::commands::proxy::state::SharedProxyState;
use axum::Extension;
use axum::Json;
use axum::extract::State;
use axum::http::StatusCode;
use axum::http::header::ACCEPT_ENCODING;
use axum::http::header::CONTENT_LENGTH;
use axum::http::header::HOST;
use axum::http::header::HeaderMap;
use axum::http::header::HeaderName;
use axum::response::IntoResponse;
use axum::response::Response;
use mockql_core::GraphQLRequest;
use mockql_core::Header;
use mockql_core::MOCK_RESPONSE_EXTENSION_KEY;
use mockql_core::MockServiceRequest;
use mockql_core::SCHEMA_EXTENSION_KEY;

pub(crate) async fn handle(
  Extension(config): Extension<SharedProxyConfig>,
  State(state): State<SharedProxyState>,
  headers: HeaderMap,
  Json(request): Json<GraphQLRequest>,
) -> Response {
  let schema_extension = request
    .extensions
    .get(MOCK_RESPONSE_EXTENSION_KEY)
    .and_then(|v| v.get(SCHEMA_EXTENSION_KEY))
    .cloned();

  let result = state
    .mock_service
    .execute(
      MockServiceRequest::builder()
        .operation(request.query)
        .and_operation_name(request.operation_name)
        .variables(request.variables)
        .and_schema_extension(schema_extension)
        .graphql_url(config.graphql_url.clone())
        .graphql_headers(extract_upstream_headers(headers))
        .provider(config.provider.clone())
        .build(),
    )
    .await;

  match result {
    Ok(response) => Json(response).into_response(),
    Err(err) => (
      StatusCode::INTERNAL_SERVER_ERROR,
      Json(serde_json::json!({
        "errors": [{"message": err.to_string()}]
      })),
    )
      .into_response(),
  }
}

/// Extracts headers suitable for forwarding to the upstream GraphQL endpoint.
/// Filters out following headers:
/// - `Host` (must match the upstream, not the proxy)
/// - `Content-Length` (body size changes after re-serialization)
/// - `Accept-Encoding` (`reqwest` will handle content decoding).
fn extract_upstream_headers(headers: HeaderMap) -> Vec<Header> {
  const FILTERED: [HeaderName; 3] = [HOST, CONTENT_LENGTH, ACCEPT_ENCODING];
  headers
    .into_iter()
    .filter_map(|(name, value)| Some((name?, value)))
    .filter(|(name, _)| !FILTERED.contains(name))
    .collect()
}
