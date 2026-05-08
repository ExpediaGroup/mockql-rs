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

use crate::graphql::GraphQLResponse;
use crate::graphql::mock_response_prompt::MockResponsePrompt;
use crate::llm_provider::ProviderError;
use std::path::Path;
use std::process::Stdio;
use std::time::Duration;
use tempfile::TempDir;
use tokio::fs;
use tokio::io::AsyncWriteExt;
use tokio::process::Command;

/// OpenAI Codex CLI provider.
#[derive(Debug, Clone)]
pub struct CodexCliProvider {
  /// Model identifier forwarded to the CLI.
  pub model: String,
}

impl CodexCliProvider {
  const BINARY: &str = "codex";

  pub(crate) async fn run(
    &self,
    prompt: &MockResponsePrompt,
    timeout_duration: Duration,
  ) -> Result<GraphQLResponse, ProviderError> {
    let prompt_markdown = prompt.to_markdown()?;
    let temp_dir = TempDir::new().map_err(|error| ProviderError::Request(error.to_string()))?;
    let output_path = temp_dir.path().join("last-message.json");

    let mut command = Command::new(Self::BINARY);
    command.args(self.get_args(&output_path));
    command
      .stdin(Stdio::piped())
      .stdout(Stdio::piped())
      .stderr(Stdio::piped())
      .kill_on_drop(true);

    let mut child = command
      .spawn()
      .map_err(|error| ProviderError::Request(format!("failed to execute Codex CLI: {error}")))?;

    {
      let mut stdin = child
        .stdin
        .take()
        .ok_or_else(|| ProviderError::Request("Codex CLI stdin was not piped".to_string()))?;

      stdin
        .write_all(prompt_markdown.as_bytes())
        .await
        .map_err(|error| ProviderError::Request(format!("Codex CLI failed while writing prompt: {error}")))?;

      stdin
        .shutdown()
        .await
        .map_err(|error| ProviderError::Request(format!("Codex CLI failed while closing stdin: {error}")))?;
    }

    let output = super::exec_cli(timeout_duration, child).await?;
    let stderr = String::from_utf8_lossy(&output.stderr);

    let output_string = fs::read_to_string(&output_path).await.map_err(|error| {
      ProviderError::Request(if stderr.trim().is_empty() {
        format!("Codex CLI did not write last-message output: {error}")
      } else {
        format!(
          "Codex CLI did not write last-message output: {error}. stderr: {}",
          stderr.trim()
        )
      })
    })?;

    Self::parse_response(&output_string)
  }

  fn get_args(&self, output_path: &Path) -> Vec<String> {
    let output_path = output_path.display().to_string();
    let args = vec![
      "exec",
      "--skip-git-repo-check",
      "--yolo",
      "-m",
      &self.model,
      "--config",
      r##"model_reasoning_effort="low""##,
      "--json",
      "--output-last-message",
      &output_path,
      "-",
    ];
    args.into_iter().map(String::from).collect()
  }

  fn parse_response(file_text: &str) -> Result<GraphQLResponse, ProviderError> {
    let trimmed = file_text.trim();
    if trimmed.is_empty() {
      return Err(ProviderError::Request(
        "Codex CLI returned an empty last-message output".to_string(),
      ));
    }
    GraphQLResponse::try_from(trimmed).map_err(ProviderError::InvalidJson)
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn builds_script_mode_args() {
    let provider = CodexCliProvider {
      model: "gpt-5.4-mini".into(),
    };
    let args = provider.get_args(Path::new("mock-response.codex.json"));
    assert_eq!(
      args,
      vec![
        "exec",
        "--skip-git-repo-check",
        "--yolo",
        "-m",
        "gpt-5.4-mini",
        "--config",
        r##"model_reasoning_effort="low""##,
        "--json",
        "--output-last-message",
        "mock-response.codex.json",
        "-",
      ]
    );
  }

  #[test]
  fn parses_fixture_last_message() {
    let response =
      CodexCliProvider::parse_response(include_str!("mock-response.codex.json")).expect("fixture should parse");

    let data = response.data.expect("response should contain data");
    assert_eq!(
      data["loyaltyRewards"]["additionalInformation"][0]["title"],
      "Platinum member benefits"
    );
  }
}
