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

mod args;
mod config;
mod handler;
mod state;

pub(crate) use self::args::ProxyArgs;
use super::super::CliError;
use crate::commands::proxy::config::ProxyConfig;
use crate::commands::proxy::state::ProxyState;
use axum::Extension;
use axum::Router;
use axum::http::StatusCode;
use axum::response::Html;
use axum::routing::get;
use axum::routing::post;
use axum::serve;
use mockql_core::MockService;
use mockql_core::load_schema;
use reqwest::Client;
use std::sync::Arc;
use tokio::net::TcpListener;

pub(crate) async fn exec_proxy(args: ProxyArgs) -> Result<(), CliError> {
  let ProxyArgs {
    port,
    graphql_url,
    schema,
    introspection_headers,
    timeout,
    format,
    transport,
  } = args;

  let client = Client::builder()
    .timeout(timeout)
    .build()
    .map_err(|e| CliError::ClientInit(e.to_string()))?;

  let schema = Arc::new(load_schema(schema.as_deref(), Some(&graphql_url), &introspection_headers, &client).await?);

  let state = ProxyState::new_shared(MockService::new(schema, format.into(), client.clone()));

  let config = ProxyConfig::new_shared(graphql_url, transport.into_provider_config(&timeout, client));

  const GRAPHIQL_HTML: &str = include_str!("graphiql.html");

  let router = Router::new()
    .route("/health", get(|| async { (StatusCode::OK, "ACTIVE") }))
    .route("/", get(|| async { Html(GRAPHIQL_HTML) }))
    .route("/graphiql", get(|| async { Html(GRAPHIQL_HTML) }))
    .route("/graphql", post(handler::handle))
    .with_state(state)
    .layer(Extension(config));

  let listener = TcpListener::bind(format!("0.0.0.0:{}", port)).await?;

  serve(listener, router).await?;

  Ok(())
}
