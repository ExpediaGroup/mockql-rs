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
use crate::llm_provider::http::github_copilot::GithubCopilotHttpProvider;
use reqwest::Client;
use std::env;

pub mod github_copilot;

/// Supported HTTP provider variants.
#[derive(Debug, Clone)]
pub enum HttpProvider {
  /// GitHub Copilot HTTP provider.
  GithubCopilot(GithubCopilotHttpProvider),
}

impl HttpProvider {
  pub(crate) async fn run(
    &self,
    prompt: &MockResponsePrompt,
    client: &Client,
  ) -> Result<GraphQLResponse, ProviderError> {
    if env::var("MOCKQL_DEBUG").is_ok()
      && let Ok(markdown) = prompt.to_markdown()
    {
      let _ = std::fs::write("prompt.dbg.txt", markdown);
    }
    match self {
      Self::GithubCopilot(github_copilot) => github_copilot.run(prompt, client).await,
    }
  }
}
