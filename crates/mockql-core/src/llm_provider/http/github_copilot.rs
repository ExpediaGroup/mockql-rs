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

use crate::GraphQLResponse;
use crate::ProviderError;
use crate::graphql::mock_response_prompt::MockResponsePrompt;
use reqwest::Client;
use serde_json::json;
use std::env;

/// GitHub Copilot HTTP provider.
#[derive(Debug, Clone)]
pub struct GithubCopilotHttpProvider {
  /// Model identifier forwarded to the GitHub Copilot request.
  pub model: String,
}

impl GithubCopilotHttpProvider {
  const ENDPOINT: &str = "https://api.githubcopilot.com/chat/completions";
  const TOKEN_ENV_VAR: &str = "GITHUB_TOKEN";

  pub(crate) async fn run(
    &self,
    prompt: &MockResponsePrompt,
    client: &Client,
  ) -> Result<GraphQLResponse, ProviderError> {
    let token =
      env::var(Self::TOKEN_ENV_VAR).map_err(|_| ProviderError::MissingEnv(Self::TOKEN_ENV_VAR.to_string()))?;
    let response = client
      .post(Self::ENDPOINT)
      .bearer_auth(token)
      .header("Content-Type", "application/json")
      .json(&json!({
        "model": self.model,
        "messages": [{ "role": "user", "content": prompt.to_markdown()? }],
        "stream": false
      }))
      .send()
      .await?;

    let status = response.status();
    let text = response.text().await?;

    if !status.is_success() {
      return Err(ProviderError::UnexpectedStatus { status, body: text });
    }

    Self::parse_response(&text)
  }

  fn parse_response(body: &str) -> Result<GraphQLResponse, ProviderError> {
    let payload: serde_json::Value = serde_json::from_str(body)?;
    let content = payload
      .get("choices")
      .and_then(|choices| choices.as_array())
      .and_then(|choices| choices.first())
      .and_then(|choice| choice.get("message"))
      .and_then(|message| message.get("content"))
      .and_then(|content| content.as_str())
      .ok_or(ProviderError::MalformedResponse("choices[0].message.content"))?;

    let trimmed = content.trim();
    GraphQLResponse::try_from(trimmed).map_err(ProviderError::InvalidJson)
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn parses_fixture_response() {
    let response = GithubCopilotHttpProvider::parse_response(include_str!("mock-response.github-copilot.json"))
      .expect("fixture should parse");

    let data = response.data.expect("response should contain data");
    assert_eq!(data["allFilms"]["films"][0]["director"], "George Lucas");
  }

  #[test]
  fn rejects_missing_content() {
    let body = json!({ "choices": [] }).to_string();
    let error = GithubCopilotHttpProvider::parse_response(&body).unwrap_err();
    assert!(matches!(
      error,
      ProviderError::MalformedResponse("choices[0].message.content")
    ));
  }

  #[test]
  fn rejects_invalid_json() {
    let error = GithubCopilotHttpProvider::parse_response("{not json}").unwrap_err();
    assert!(matches!(error, ProviderError::InvalidJson(_)));
  }
}
