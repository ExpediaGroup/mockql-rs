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
pub(crate) struct ProxyArgs {
  /// Port for the mock GraphQL server.
  #[arg(long)]
  pub(super) port: u16,

  /// GraphQL server URL used for schema introspection and upstream execution.
  #[arg(long, value_parser = clap::value_parser!(Url))]
  pub(super) graphql_url: Url,

  /// Optional path to a GraphQL schema file. If omitted, the schema is loaded from `--graphql-url` via introspection.
  #[arg(long)]
  pub(super) schema: Option<PathBuf>,

  /// Additional headers to include in the introspection query if no local schema was provided. Specify as
  /// `name:value`; repeat the flag for multiple headers.
  #[arg(long = "introspection-header", value_parser = parse_header)]
  pub(super) introspection_headers: Vec<Header>,

  /// Timeout for provider execution, e.g. `30s` or `2m`. Applied as a per-invocation timeout
  /// for CLI providers, and as the underlying reqwest Client timeout for HTTP providers.
  #[arg(long, default_value = "1m", value_parser = humantime::parse_duration)]
  pub(super) timeout: Duration,

  /// Serialization format for the mock response prompt. Defaults to JSON.
  #[arg(long, value_enum, default_value = "json")]
  pub(super) format: SerializationFormatArg,

  /// Transport and provider to use (subcommand).
  #[command(subcommand)]
  pub(super) transport: TransportArg,
}

#[cfg(test)]
mod tests {
  use crate::Commands;
  use crate::MockQLCli;
  use crate::commands::shared::HttpProviderArg;
  use crate::commands::shared::SerializationFormatArg;
  use crate::commands::shared::TransportArg;
  use clap::Parser;
  use clap::error::ErrorKind;
  use reqwest::header::HeaderName;
  use reqwest::header::HeaderValue;
  use std::time::Duration;

  #[test]
  fn proxy_requires_port() {
    let result = MockQLCli::try_parse_from([
      "mockql",
      "proxy",
      "--graphql-url",
      "https://example.com/graphql",
      "cli",
      "--provider",
      "codex",
    ]);
    assert_eq!(result.unwrap_err().kind(), ErrorKind::MissingRequiredArgument);
  }

  #[test]
  fn proxy_requires_graphql_url() {
    let result = MockQLCli::try_parse_from(["mockql", "proxy", "--port", "8080", "cli", "--provider", "codex"]);
    assert_eq!(result.unwrap_err().kind(), ErrorKind::MissingRequiredArgument);
  }

  #[test]
  fn proxy_rejects_an_invalid_graphql_url() {
    let result = MockQLCli::try_parse_from([
      "mockql",
      "proxy",
      "--port",
      "8080",
      "--graphql-url",
      "not-a-url",
      "cli",
      "--provider",
      "codex",
    ]);
    assert_eq!(result.unwrap_err().kind(), ErrorKind::ValueValidation);
  }

  #[test]
  fn proxy_defaults_model_for_claude() {
    let app = MockQLCli::try_parse_from([
      "mockql",
      "proxy",
      "--port",
      "8080",
      "--graphql-url",
      "https://example.com/graphql",
      "cli",
      "--provider",
      "claude",
    ])
    .unwrap();

    let Commands::Proxy(args) = app.command else {
      panic!("expected to parse proxy command");
    };
    let TransportArg::Cli { model, .. } = args.transport else {
      panic!("expected cli transport variant");
    };
    assert_eq!(model, "sonnet");
  }

  #[test]
  fn proxy_parses_introspection_headers_timeout_and_format() {
    let app = MockQLCli::try_parse_from([
      "mockql",
      "proxy",
      "--port",
      "8080",
      "--graphql-url",
      "https://example.com/graphql",
      "--introspection-header",
      "client-info:test",
      "--timeout",
      "30s",
      "--format",
      "toon",
      "cli",
      "--provider",
      "codex",
    ])
    .unwrap();

    let Commands::Proxy(args) = app.command else {
      panic!("expected to parse proxy command");
    };
    assert_eq!(args.port, 8080);
    assert_eq!(args.graphql_url.as_str(), "https://example.com/graphql");
    assert_eq!(args.introspection_headers.len(), 1);
    assert_eq!(args.introspection_headers[0].0, HeaderName::from_static("client-info"));
    assert_eq!(args.introspection_headers[0].1, HeaderValue::from_static("test"));
    assert_eq!(args.timeout, Duration::from_secs(30));
    assert!(matches!(args.format, SerializationFormatArg::Toon));
  }

  #[test]
  fn proxy_http_parses_github_copilot_provider() {
    let app = MockQLCli::try_parse_from([
      "mockql",
      "proxy",
      "--port",
      "8080",
      "--graphql-url",
      "https://example.com/graphql",
      "http",
      "github-copilot",
      "--model",
      "gpt-5",
    ])
    .unwrap();

    let Commands::Proxy(args) = app.command else {
      panic!("expected to parse proxy command");
    };
    let TransportArg::Http(HttpProviderArg::GithubCopilot { model }) = args.transport else {
      panic!("expected GitHub Copilot HTTP provider");
    };
    assert_eq!(model, "gpt-5");
  }

  #[test]
  fn proxy_http_parses_gemini_compatible() {
    let app = MockQLCli::try_parse_from([
      "mockql",
      "proxy",
      "--port",
      "8080",
      "--graphql-url",
      "https://example.com/graphql",
      "http",
      "gemini",
      "--url",
      "https://generativelanguage.googleapis.com/v1beta/models/gemini-3.5-flash:generateContent",
      "--auth-header",
      "x-goog-api-key",
      "--auth-value-env-var",
      "CUSTOM_API_KEY",
    ])
    .unwrap();

    let Commands::Proxy(args) = app.command else {
      panic!("expected to parse proxy command");
    };
    let TransportArg::Http(HttpProviderArg::Gemini {
      url,
      auth_header,
      auth_value_env_var,
    }) = args.transport
    else {
      panic!("expected Gemini compatible endpoint HTTP provider");
    };
    assert_eq!(
      url.as_str(),
      "https://generativelanguage.googleapis.com/v1beta/models/gemini-3.5-flash:generateContent"
    );
    assert_eq!(auth_header, HeaderName::from_static("x-goog-api-key"));
    assert_eq!(auth_value_env_var, "CUSTOM_API_KEY");
  }
}
