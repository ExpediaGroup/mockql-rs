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

//! Provider abstractions and built-in provider clients.

pub mod cli;
pub mod http;

use self::cli::CliProvider;
use crate::graphql::GraphQLResponse;
use crate::graphql::mock_response_prompt::MockResponsePrompt;
use crate::graphql::mock_response_prompt::PromptSerializationError;
use crate::llm_provider::http::HttpProvider;
use reqwest::Client;
use std::time::Duration;
use thiserror::Error;

/// Supported provider transport configurations.
#[derive(Debug, Clone)]
pub enum ProviderConfig {
  /// CLI provider configuration.
  Cli {
    /// CLI provider variant to invoke.
    provider: CliProvider,
    /// Maximum amount of time allowed for the CLI invocation.
    timeout: Duration,
  },
  /// HTTP provider configuration.
  Http {
    /// HTTP provider variant to invoke.
    provider: HttpProvider,
    /// HTTP client used to issue provider requests. Its configured timeout applies.
    client: Client,
  },
}

/// Errors raised while serializing prompts or invoking mock providers.
#[derive(Debug, Error)]
pub enum ProviderError {
  /// Prompt serialization failed before the request could be sent.
  #[error("failed to serialize prompt: {0}")]
  Prompt(#[from] PromptSerializationError),
  /// The provider could not be invoked or returned an execution error.
  #[error("provider request failed: {0}")]
  Request(String),
  /// A required environment variable was not set.
  #[error("missing environment variable: {0}")]
  MissingEnv(String),
  /// HTTP transport failure while invoking an HTTP provider.
  #[error(transparent)]
  Http(#[from] reqwest::Error),
  /// The provider returned a non-success HTTP status.
  #[error("provider returned status {status}: {body}")]
  UnexpectedStatus {
    /// HTTP status code returned by the provider.
    status: reqwest::StatusCode,
    /// Raw response body, if any.
    body: String,
  },
  /// The provider response was missing an expected field.
  #[error("malformed provider response: missing {0}")]
  MalformedResponse(&'static str),
  /// The provider returned invalid JSON.
  #[error("provider returned invalid JSON: {0}")]
  InvalidJson(#[from] serde_json::Error),
}

/// Dispatches a mock response prompt to a provider.
pub async fn generate_mock_response(
  config: &ProviderConfig,
  prompt: &MockResponsePrompt,
) -> Result<GraphQLResponse, ProviderError> {
  match config {
    ProviderConfig::Cli { provider, timeout } => provider.run(prompt, *timeout).await,
    ProviderConfig::Http { provider, client } => provider.run(prompt, client).await,
  }
}
