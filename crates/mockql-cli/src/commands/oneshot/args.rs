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

use crate::commands::shared::SerializationFormatArg;
use crate::commands::shared::TransportArg;
use crate::commands::shared::parse_header;
use clap::Args;
use mockql_core::Header;
use reqwest::Url;
use std::path::PathBuf;
use std::time::Duration;

#[derive(Debug, Clone, Args)]
pub(crate) struct OneshotArgs {
  /// GraphQL operation path to execute.
  #[arg(long)]
  pub(super) operation: PathBuf,

  /// GraphQL server URL used for schema introspection and upstream execution.
  #[arg(long, value_parser = clap::value_parser!(Url))]
  pub(super) graphql_url: Url,

  /// Optional path to a GraphQL schema file. If omitted, the schema is loaded from `--graphql-url` via introspection.
  #[arg(long)]
  pub(super) schema: Option<PathBuf>,

  /// Optional operation name for documents with multiple operation definitions.
  #[arg(long)]
  pub(super) operation_name: Option<String>,

  /// Optional path to a JSON file containing variables for the operation.
  #[arg(long)]
  pub(super) variables: Option<PathBuf>,

  /// Optional path to a GraphQL schema extension file for mocking fields or types absent from the base schema.
  #[arg(long)]
  pub(super) schema_extension: Option<PathBuf>,

  /// Additional headers to include in GraphQL requests. Specify as `name:value`; repeat the flag for multiple headers.
  #[arg(long = "graphql-header", value_parser = parse_header)]
  pub(super) graphql_headers: Vec<Header>,

  /// Timeout for provider execution, e.g. `30s` or `2m`. Applied as a per-invocation timeout
  /// for CLI providers, and as the underlying reqwest Client timeout for HTTP providers.
  #[arg(long, default_value = "1m", value_parser = humantime::parse_duration)]
  pub(super) timeout: Duration,

  /// Serialization format for the mock response prompt. Defaults to JSON.
  #[arg(long, value_enum, default_value = "json")]
  pub(super) format: SerializationFormatArg,

  /// LLM provider transport to use.
  #[command(subcommand)]
  pub(super) transport: TransportArg,
}

#[cfg(test)]
mod tests {
  use crate::Commands;
  use crate::MockQLCli;
  use crate::commands::shared::HttpProviderArg;
  use crate::commands::shared::TransportArg;
  use crate::commands::shared::parse_header;
  use clap::Parser;
  use clap::error::ErrorKind;
  use reqwest::header::HeaderName;
  use reqwest::header::HeaderValue;

  #[test]
  fn oneshot_requires_graphql_url() {
    let result = MockQLCli::try_parse_from([
      "mockql",
      "oneshot",
      "--operation",
      "op.graphql",
      "cli",
      "--provider",
      "codex",
    ]);
    assert_eq!(result.unwrap_err().kind(), ErrorKind::MissingRequiredArgument);
  }

  #[test]
  fn oneshot_requires_a_cli_provider_value() {
    let result = MockQLCli::try_parse_from([
      "mockql",
      "oneshot",
      "--operation",
      "op.graphql",
      "--graphql-url",
      "https://example.com/graphql",
      "cli",
      "--provider",
    ]);
    assert_eq!(result.unwrap_err().kind(), ErrorKind::InvalidValue);
  }

  #[test]
  fn oneshot_rejects_an_invalid_graphql_url() {
    let result = MockQLCli::try_parse_from([
      "mockql",
      "oneshot",
      "--operation",
      "op.graphql",
      "--graphql-url",
      "not-a-url",
      "cli",
      "--provider",
      "codex",
    ]);
    assert_eq!(result.unwrap_err().kind(), ErrorKind::ValueValidation);
  }

  #[test]
  fn oneshot_defaults_model_for_claude() {
    let app = MockQLCli::try_parse_from([
      "mockql",
      "oneshot",
      "--operation",
      "op.graphql",
      "--graphql-url",
      "https://example.com/graphql",
      "cli",
      "--provider",
      "claude",
    ])
    .unwrap();

    let Commands::Oneshot(args) = app.command else {
      panic!("expected to parse oneshot command");
    };
    let TransportArg::Cli { model, .. } = args.transport else {
      panic!("expected cli transport variant");
    };
    assert_eq!(model, "sonnet");
  }

  #[test]
  fn oneshot_defaults_model_for_codex() {
    let app = MockQLCli::try_parse_from([
      "mockql",
      "oneshot",
      "--operation",
      "op.graphql",
      "--graphql-url",
      "https://example.com/graphql",
      "cli",
      "--provider",
      "codex",
    ])
    .unwrap();

    let Commands::Oneshot(args) = app.command else {
      panic!("expected to parse oneshot command");
    };
    let TransportArg::Cli { model, .. } = args.transport else {
      panic!("expected cli transport variant");
    };
    assert_eq!(model, "gpt-5.4-mini");
  }

  #[test]
  fn oneshot_allows_explicit_model_override() {
    let app = MockQLCli::try_parse_from([
      "mockql",
      "oneshot",
      "--operation",
      "op.graphql",
      "--graphql-url",
      "https://example.com/graphql",
      "cli",
      "--provider",
      "claude",
      "--model",
      "opus",
    ])
    .unwrap();

    let Commands::Oneshot(args) = app.command else {
      panic!("expected to parse oneshot command");
    };
    let TransportArg::Cli { model, .. } = args.transport else {
      panic!("expected cli transport variant");
    };
    assert_eq!(model, "opus");
  }

  #[test]
  fn parse_header_parses_typed_header_name_and_value() {
    let header = parse_header("client-info: test").expect("header should parse");
    assert_eq!(header.0, HeaderName::from_static("client-info"));
    assert_eq!(header.1, HeaderValue::from_static("test"));
  }

  #[test]
  fn oneshot_http_requires_model() {
    let result = MockQLCli::try_parse_from([
      "mockql",
      "oneshot",
      "--operation",
      "op.graphql",
      "--graphql-url",
      "https://example.com/graphql",
      "http",
      "github-copilot",
    ]);
    assert_eq!(result.unwrap_err().kind(), ErrorKind::MissingRequiredArgument);
  }

  #[test]
  fn oneshot_http_parses_github_copilot_provider() {
    let app = MockQLCli::try_parse_from([
      "mockql",
      "oneshot",
      "--operation",
      "op.graphql",
      "--graphql-url",
      "https://example.com/graphql",
      "http",
      "github-copilot",
      "--model",
      "gpt-5",
    ])
    .unwrap();

    let Commands::Oneshot(args) = app.command else {
      panic!("expected to parse oneshot command");
    };
    let TransportArg::Http(HttpProviderArg::GithubCopilot { model }) = args.transport else {
      panic!("expected GitHub Copilot HTTP provider");
    };
    assert_eq!(model, "gpt-5");
  }

  #[test]
  fn oneshot_http_parses_gemini_compatible() {
    let app = MockQLCli::try_parse_from([
      "mockql",
      "oneshot",
      "--operation",
      "op.graphql",
      "--graphql-url",
      "https://example.com/graphql",
      "http",
      "gemini",
      "--url",
      "https://generativelanguage.googleapis.com/v1beta/models/gemini-3.5-flash:generateContent",
      "--auth-header",
      "x-goog-api-key",
    ])
    .unwrap();

    let Commands::Oneshot(args) = app.command else {
      panic!("expected to parse oneshot command");
    };
    let TransportArg::Http(HttpProviderArg::Gemini { url, auth_header }) = args.transport else {
      panic!("expected Gemini compatible endpoint HTTP provider");
    };
    assert_eq!(
      url.as_str(),
      "https://generativelanguage.googleapis.com/v1beta/models/gemini-3.5-flash:generateContent"
    );
    assert_eq!(auth_header, HeaderName::from_static("x-goog-api-key"));
  }
}
