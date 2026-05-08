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

//! CLI-backed provider client implementations.

mod claude;
mod codex;
mod opencode;

use crate::GraphQLResponse;
use crate::graphql::mock_response_prompt::MockResponsePrompt;
use crate::llm_provider::ProviderError;
pub use claude::ClaudeCliProvider;
pub use codex::CodexCliProvider;
pub use opencode::OpenCodeCliProvider;
use std::env;
use std::process::Output;
use std::time::Duration;
use tokio::process::Child;
use tokio::time::timeout;

/// Supported local CLI providers.
#[derive(Debug, Clone)]
pub enum CliProvider {
  /// Anthropic Claude CLI.
  Claude(ClaudeCliProvider),
  /// OpenAI Codex CLI.
  Codex(CodexCliProvider),
  /// Anomaly OpenCode CLI.
  OpenCode(OpenCodeCliProvider),
}

impl CliProvider {
  pub(crate) async fn run(
    &self,
    prompt: &MockResponsePrompt,
    timeout: Duration,
  ) -> Result<GraphQLResponse, ProviderError> {
    if env::var("MOCKQL_DEBUG").is_ok()
      && let Ok(markdown) = prompt.to_markdown()
    {
      let _ = std::fs::write("prompt.dbg.txt", markdown);
    }
    match self {
      Self::Claude(claude) => claude.run(prompt, timeout).await,
      Self::Codex(codex) => codex.run(prompt, timeout).await,
      Self::OpenCode(opencode) => opencode.run(prompt, timeout).await,
    }
  }
}

/// Waits for a spawned CLI child process with a timeout and checks its exit status.
pub(crate) async fn exec_cli(timeout_duration: Duration, child: Child) -> Result<Output, ProviderError> {
  let output = timeout(timeout_duration, child.wait_with_output())
    .await
    .map_err(|_| ProviderError::Request("CLI command timed out".to_string()))?
    .map_err(|error| ProviderError::Request(format!("CLI command failed while waiting for output: {error}")))?;

  if !output.status.success() {
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    let details = if stderr.trim().is_empty() {
      stdout.trim()
    } else {
      stderr.trim()
    };
    return Err(ProviderError::Request(if details.is_empty() {
      format!("CLI command exited with status {}", output.status)
    } else {
      format!("CLI command exited with status {}: {details}", output.status)
    }));
  }

  Ok(output)
}
