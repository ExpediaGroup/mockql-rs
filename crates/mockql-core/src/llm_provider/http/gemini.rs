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

use crate::graphql::mock_response_prompt::MockResponsePrompt;
use crate::{GraphQLResponse, ProviderError};
use reqwest::header::HeaderName;
use reqwest::{Client, Url};
use serde_json::json;
use std::env;

/// Gemini Compatible Endpoint HTTP provider.
#[derive(Debug, Clone)]
pub struct GeminiCompatibleHttpProvider {
  /// Full endpoint URL.
  pub url: Url,
  /// Header name used for the auth value loaded from the static env var.
  pub auth_header: HeaderName,
}

impl GeminiCompatibleHttpProvider {
  const TOKEN_ENV_VAR: &str = "AUTH_TOKEN";

  pub(crate) async fn run(
    &self,
    prompt: &MockResponsePrompt,
    client: &Client,
  ) -> Result<GraphQLResponse, ProviderError> {
    let token = env::var(Self::TOKEN_ENV_VAR).map_err(|_| ProviderError::MissingEnv(Self::TOKEN_ENV_VAR))?;

    let response = client
      .post(self.url.clone())
      .header(self.auth_header.clone(), token)
      .header("Content-Type", "application/json")
      .json(&json!({
        "contents": [{
          "role": "user",
          "parts": [{ "text": prompt.to_markdown()? }]
        }]
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
      .get("candidates")
      .and_then(|candidates| candidates.as_array())
      .and_then(|candidates| candidates.first())
      .and_then(|candidate| candidate.get("content"))
      .and_then(|content| content.get("parts"))
      .and_then(|parts| parts.as_array())
      .and_then(|parts| {
        parts
          .iter()
          .find_map(|part| part.get("text").and_then(|text| text.as_str()))
      })
      .ok_or(ProviderError::MalformedResponse("candidates[0].content.parts[0].text"))?;

    GraphQLResponse::try_from(content.trim()).map_err(ProviderError::InvalidJson)
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn parses_fixture_response() {
    let body = json!({
      "candidates": [{
        "content": {
          "parts": [{
            "text": r#"{"data":{"allFilms":{"films":[{"director":"George Lucas"}]}}}"#
          }]
        }
      }]
    })
    .to_string();

    let response = GeminiCompatibleHttpProvider::parse_response(&body).expect("fixture should parse");

    let data = response.data.expect("response should contain data");
    assert_eq!(data["allFilms"]["films"][0]["director"], "George Lucas");
  }

  #[test]
  fn rejects_missing_content() {
    let body = json!({ "candidates": [] }).to_string();
    let error = GeminiCompatibleHttpProvider::parse_response(&body).unwrap_err();
    assert!(matches!(
      error,
      ProviderError::MalformedResponse("candidates[0].content.parts[0].text")
    ));
  }

  #[test]
  fn rejects_invalid_json() {
    let error = GeminiCompatibleHttpProvider::parse_response("{not json}").unwrap_err();
    assert!(matches!(error, ProviderError::InvalidJson(_)));
  }
}
