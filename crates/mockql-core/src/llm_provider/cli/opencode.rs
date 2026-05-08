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
use std::process::Stdio;
use std::time::Duration;
use tokio::process::Command;

/// Anomaly OpenCode CLI provider.
#[derive(Debug, Clone)]
pub struct OpenCodeCliProvider {
  /// Model identifier forwarded to the CLI in the form of provider/model.
  pub model: String,
}

impl OpenCodeCliProvider {
  const BINARY: &str = "opencode";

  /// Runs the OpenCode CLI, passing the rendered prompt as a positional argument
  /// to `opencode run` rather than through stdin.
  pub(crate) async fn run(
    &self,
    prompt: &MockResponsePrompt,
    timeout_duration: Duration,
  ) -> Result<GraphQLResponse, ProviderError> {
    let prompt_markdown = prompt.to_markdown()?;

    let mut command = Command::new(Self::BINARY);
    command.args(self.get_args(prompt_markdown));
    command.stdout(Stdio::piped()).stderr(Stdio::piped()).kill_on_drop(true);

    let child = command
      .spawn()
      .map_err(|error| ProviderError::Request(format!("failed to execute OpenCode CLI: {error}")))?;

    let output = super::exec_cli(timeout_duration, child).await?;
    let stdout = String::from_utf8_lossy(&output.stdout);
    Self::parse_response(stdout.as_ref())
  }

  fn get_args(&self, prompt: String) -> Vec<String> {
    vec!["run".into(), prompt, "--model".into(), self.model.clone()]
  }

  fn parse_response(output: &str) -> Result<GraphQLResponse, ProviderError> {
    let trimmed = output.trim();
    if trimmed.is_empty() {
      return Err(ProviderError::Request("OpenCode CLI returned empty output".to_string()));
    }
    GraphQLResponse::try_from(trimmed).map_err(ProviderError::InvalidJson)
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn builds_run_args() {
    let provider = OpenCodeCliProvider {
      model: "github-copilot/gemini-3-flash-preview".into(),
    };
    let args = provider.get_args("test prompt".into());
    assert_eq!(
      args,
      vec!["run", "test prompt", "--model", "github-copilot/gemini-3-flash-preview"]
    );
  }

  #[test]
  fn parses_fixture_response() {
    let response =
      OpenCodeCliProvider::parse_response(include_str!("mock-response.opencode.json")).expect("fixture should parse");

    let data = response.data.expect("response should contain data");
    assert_eq!(data["allFilms"]["films"][0]["director"], "George Lucas");
  }

  #[test]
  fn rejects_empty_output() {
    let error = OpenCodeCliProvider::parse_response("").unwrap_err();
    assert!(error.to_string().contains("empty output"));
  }
}
