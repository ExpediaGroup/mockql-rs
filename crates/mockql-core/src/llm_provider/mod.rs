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

/// Parses a `GraphQLResponse` from raw LLM text, tolerating markdown fences, leading prose, and trailing junk.
pub(crate) fn parse_llm_json(raw: &str) -> Result<GraphQLResponse, ProviderError> {
  let trimmed = raw.trim();
  if let Ok(response) = GraphQLResponse::try_from(trimmed) {
    return Ok(response);
  }

  // ponytail: O(n) scan over `{` candidates; avoids latching onto non-GraphQL `{...}` in prose
  let mut first_err = None;
  for (idx, _) in trimmed.match_indices('{') {
    match serde_json::Deserializer::from_str(&trimmed[idx..])
      .into_iter::<GraphQLResponse>()
      .next()
    {
      Some(Ok(response)) if response.data.is_some() || !response.errors.is_empty() => return Ok(response),
      Some(Err(err)) if first_err.is_none() => first_err = Some(err),
      _ => {}
    }
  }

  Err(ProviderError::InvalidJson(
    first_err.unwrap_or_else(|| GraphQLResponse::try_from(trimmed).unwrap_err()),
  ))
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn parse_llm_json_handles_clean_fenced_prose_and_trailing() {
    let cases = [
      r#"{"data":{"ok":true}}"#,
      "```json\n{\"data\":{\"ok\":true}}\n```",
      "Here is the {example} output:\n{\"data\":{\"ok\":true}}\nHope that helps!",
      "{\"data\":{\"ok\":true}}} trailing",
    ];
    for raw in cases {
      let resp = parse_llm_json(raw).unwrap_or_else(|e| panic!("failed on {raw:?}: {e}"));
      assert_eq!(resp.data.unwrap()["ok"], true);
    }
  }

  #[test]
  fn parse_llm_json_rejects_invalid_json() {
    assert!(matches!(parse_llm_json("not json"), Err(ProviderError::InvalidJson(_))));
    assert!(matches!(
      parse_llm_json("{\"data\":{\"ok\":"),
      Err(ProviderError::InvalidJson(_))
    ));
  }
}
