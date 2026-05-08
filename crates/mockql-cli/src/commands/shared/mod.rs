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

use crate::CliError;
use clap::Subcommand;
use clap::ValueEnum;
use mockql_core::ClaudeCliProvider;
use mockql_core::CliProvider;
use mockql_core::CodexCliProvider;
use mockql_core::GithubCopilotHttpProvider;
use mockql_core::Header;
use mockql_core::HttpProvider;
use mockql_core::OpenCodeCliProvider;
use mockql_core::ProviderConfig;
use mockql_core::SerializationFormat;
use reqwest::Client;
use reqwest::header::HeaderName;
use reqwest::header::HeaderValue;
use serde_json_bytes::ByteString;
use serde_json_bytes::Map;
use serde_json_bytes::Value;
use std::fs;
use std::path;
use std::time::Duration;

#[derive(Debug, Clone, Subcommand)]
pub(crate) enum TransportArg {
  /// Use a local CLI provider (claude, codex, opencode).
  Cli {
    /// CLI provider to use.
    #[arg(long, value_enum)]
    provider: CliProviderArg,

    /// Model name forwarded to the CLI provider. Defaults to `sonnet` for `claude`,
    /// `gpt-5.4-mini` for `codex`, and `github-copilot/gemini-3-flash-preview` for `opencode`.
    #[arg(
      long,
      required = false,
      default_value_if("provider", "claude", Some("sonnet")),
      default_value_if("provider", "codex", Some("gpt-5.4-mini")),
      default_value_if("provider", "opencode", Some("github-copilot/gemini-3-flash-preview"))
    )]
    model: String,
  },
  /// Use an HTTP provider (github-copilot).
  Http {
    /// HTTP provider to use.
    #[arg(long, value_enum)]
    provider: HttpProviderArg,

    /// Model name forwarded to the HTTP provider.
    #[arg(long)]
    model: String,
  },
}

#[derive(Debug, Clone, Copy, ValueEnum, PartialEq)]
pub(crate) enum CliProviderArg {
  Claude,
  Codex,
  #[value(name = "opencode")]
  OpenCode,
}

#[derive(Debug, Clone, Copy, ValueEnum, PartialEq)]
pub(crate) enum HttpProviderArg {
  GithubCopilot,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
pub(super) enum SerializationFormatArg {
  Json,
  Toon,
}

impl From<SerializationFormatArg> for SerializationFormat {
  fn from(value: SerializationFormatArg) -> Self {
    match value {
      SerializationFormatArg::Json => SerializationFormat::Json,
      SerializationFormatArg::Toon => SerializationFormat::Toon,
    }
  }
}

impl TransportArg {
  pub(super) fn into_provider_config(self, timeout: &Duration, client: Client) -> ProviderConfig {
    match self {
      Self::Cli { provider, model } => ProviderConfig::Cli {
        provider: match provider {
          CliProviderArg::Claude => CliProvider::Claude(ClaudeCliProvider { model }),
          CliProviderArg::Codex => CliProvider::Codex(CodexCliProvider { model }),
          CliProviderArg::OpenCode => CliProvider::OpenCode(OpenCodeCliProvider { model }),
        },
        timeout: *timeout,
      },
      Self::Http { provider, model } => ProviderConfig::Http {
        provider: match provider {
          HttpProviderArg::GithubCopilot => HttpProvider::GithubCopilot(GithubCopilotHttpProvider { model }),
        },
        client,
      },
    }
  }
}

pub(super) fn parse_header(input: &str) -> Result<Header, String> {
  let (name, value) = input
    .split_once(':')
    .ok_or_else(|| "headers must use name:value".to_string())?;
  let name = HeaderName::from_bytes(name.trim().as_bytes()).map_err(|e| e.to_string())?;
  let value = HeaderValue::from_str(value.trim()).map_err(|e| e.to_string())?;
  Ok((name, value))
}

pub(super) fn read_json_map(path: Option<&path::Path>) -> Result<Map<ByteString, Value>, CliError> {
  let Some(path) = path else {
    return Ok(Map::new());
  };
  let content = fs::read_to_string(path)?;
  let value: serde_json::Value = serde_json::from_str(&content)?;
  let value = serde_json_bytes::to_value(value)?;
  match value {
    Value::Object(map) => Ok(map),
    _ => Err(CliError::Message("variables JSON must be an object".to_string())),
  }
}

pub(super) fn read_schema_extension(path: Option<&path::Path>) -> Result<Option<Value>, CliError> {
  let Some(path) = path else {
    return Ok(None);
  };
  let content = std::fs::read_to_string(path)?;
  Ok(Some(Value::String(content.into())))
}
