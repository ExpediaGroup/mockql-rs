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
use serde_json::Value;
use std::process::Stdio;
use std::time::Duration;
use tokio::io::AsyncWriteExt;
use tokio::process::Command;

/// Anthropic Claude CLI provider.
#[derive(Debug, Clone)]
pub struct ClaudeCliProvider {
  /// Model identifier forwarded to the CLI.
  pub model: String,
}

impl ClaudeCliProvider {
  const BINARY: &str = "claude";

  pub(crate) async fn run(
    &self,
    prompt: &MockResponsePrompt,
    timeout_duration: Duration,
  ) -> Result<GraphQLResponse, ProviderError> {
    let prompt_markdown = prompt.to_markdown()?;

    let mut command = Command::new(Self::BINARY);
    command.args(self.get_args());
    command
      .stdin(Stdio::piped())
      .stdout(Stdio::piped())
      .stderr(Stdio::piped())
      .kill_on_drop(true);

    let mut child = command
      .spawn()
      .map_err(|error| ProviderError::Request(format!("failed to execute Claude CLI: {error}")))?;

    {
      let mut stdin = child
        .stdin
        .take()
        .ok_or_else(|| ProviderError::Request("Claude CLI stdin was not piped".to_string()))?;

      stdin
        .write_all(prompt_markdown.as_bytes())
        .await
        .map_err(|error| ProviderError::Request(format!("Claude CLI failed while writing prompt: {error}")))?;

      stdin
        .shutdown()
        .await
        .map_err(|error| ProviderError::Request(format!("Claude CLI failed while closing stdin: {error}")))?;
    }

    let output = super::exec_cli(timeout_duration, child).await?;
    let stdout = String::from_utf8_lossy(&output.stdout);
    Self::parse_response(stdout.as_ref())
  }

  fn get_args(&self) -> Vec<&str> {
    vec![
      "--print",
      "--dangerously-skip-permissions",
      "--model",
      &self.model,
      "--effort",
      "low",
      "--output-format",
      "json",
      "-",
    ]
  }

  fn parse_response(output: &str) -> Result<GraphQLResponse, ProviderError> {
    let trimmed = output.trim();
    if trimmed.is_empty() {
      return Err(ProviderError::Request("Claude CLI returned empty output".to_string()));
    }

    let response = serde_json::from_str::<Value>(trimmed)
      .map_err(|error| ProviderError::Request(format!("Claude CLI returned non-JSON output: {error}")))?;

    if Self::is_error(&response) {
      let details = response
        .get("result")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or_default();
      return Err(ProviderError::Request(format!(
        "Claude CLI returned an error response: {details}"
      )));
    }

    let result = response
      .get("result")
      .and_then(Value::as_str)
      .map(str::trim)
      .filter(|value| !value.is_empty())
      .ok_or_else(|| ProviderError::Request("Claude CLI JSON output did not contain a result".to_string()))?;

    GraphQLResponse::try_from(result).map_err(ProviderError::InvalidJson)
  }

  fn is_error(payload: &Value) -> bool {
    payload.get("is_error").and_then(Value::as_bool) == Some(true)
      || matches!(payload.get("subtype").and_then(Value::as_str), Some("error"))
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn builds_script_mode_args() {
    let provider = ClaudeCliProvider { model: "opus".into() };
    let args = provider.get_args();
    assert_eq!(
      args,
      vec![
        "--print",
        "--dangerously-skip-permissions",
        "--model",
        "opus",
        "--effort",
        "low",
        "--output-format",
        "json",
        "-",
      ]
    );
  }

  #[test]
  fn parses_fixture_envelope() {
    let response =
      ClaudeCliProvider::parse_response(include_str!("mock-response.claude.json")).expect("fixture should parse");

    let data = response.data.expect("response should contain data");
    assert_eq!(
      data["loyaltyRewards"]["additionalInformation"][0]["title"],
      "Platinum Member Benefits"
    );
  }

  #[test]
  fn rejects_missing_result() {
    let error =
      ClaudeCliProvider::parse_response(r#"{"type":"result","subtype":"success","is_error":false}"#).unwrap_err();
    assert!(error.to_string().contains("did not contain a result"));
  }

  #[test]
  fn rejects_invalid_json() {
    let error = ClaudeCliProvider::parse_response("{not json}").unwrap_err();
    assert!(error.to_string().contains("non-JSON output"));
  }

  #[test]
  fn rejects_error_envelope() {
    let error = ClaudeCliProvider::parse_response(
      r#"{"type":"result","subtype":"error","is_error":true,"result":"permission denied"}"#,
    )
    .unwrap_err();
    assert!(error.to_string().contains("permission denied"));
  }
}
