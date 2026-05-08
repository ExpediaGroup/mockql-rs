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

use crate::commands::shared::parse_header;
use clap::Args;
use mockql_core::Header;
use reqwest::Url;
use std::path::PathBuf;
use std::time::Duration;

#[derive(Debug, Clone, Args)]
pub(crate) struct SchemaArgs {
  /// Minify Schema SDL into a token friendy representation
  #[arg(long)]
  pub(super) minify: bool,
  /// Path to GraphQL schema file.
  #[arg(long, required_unless_present = "graphql_url", conflicts_with = "graphql_url")]
  pub(super) schema: Option<PathBuf>,

  /// GraphQL server URL used for schema introspection.
  #[arg(long, required_unless_present = "schema", value_parser = clap::value_parser!(Url))]
  pub(super) graphql_url: Option<Url>,

  /// Headers to include in the introspection query, specify as `name:value`, repeat the flag for multiple headers.
  #[arg(long = "header", value_parser = parse_header, requires = "graphql_url", conflicts_with = "schema")]
  pub(super) headers: Vec<Header>,

  /// Timeout for introspection query, e.g. `30s` or `2m`.
  #[arg(long, default_value = "1m", value_parser = humantime::parse_duration)]
  pub(super) timeout: Duration,
}

#[cfg(test)]
mod tests {
  use crate::Commands;
  use crate::MockQLCli;
  use clap::Parser;
  use clap::error::ErrorKind;
  use reqwest::header::HeaderName;
  use reqwest::header::HeaderValue;
  use std::path::PathBuf;
  use std::time::Duration;

  #[test]
  fn schema_requires_schema_or_graphql_url() {
    let result = MockQLCli::try_parse_from(["mockql", "schema"]);
    assert_eq!(result.unwrap_err().kind(), ErrorKind::MissingRequiredArgument);
  }

  #[test]
  fn schema_rejects_schema_and_graphql_url_together() {
    let result = MockQLCli::try_parse_from([
      "mockql",
      "schema",
      "--schema",
      "schema.graphql",
      "--graphql-url",
      "https://example.com/graphql",
    ]);
    assert_eq!(result.unwrap_err().kind(), ErrorKind::ArgumentConflict);
  }

  #[test]
  fn schema_rejects_an_invalid_graphql_url() {
    let result = MockQLCli::try_parse_from(["mockql", "schema", "--graphql-url", "not-a-url"]);
    assert_eq!(result.unwrap_err().kind(), ErrorKind::ValueValidation);
  }

  #[test]
  fn schema_rejects_header_with_local_schema() {
    let result = MockQLCli::try_parse_from([
      "mockql",
      "schema",
      "--schema",
      "schema.graphql",
      "--header",
      "client-info:test",
    ]);
    assert_eq!(result.unwrap_err().kind(), ErrorKind::ArgumentConflict);
  }

  #[test]
  fn schema_parses_local_schema_with_default_timeout() {
    let app = MockQLCli::try_parse_from(["mockql", "schema", "--schema", "schema.graphql"]).unwrap();

    let Commands::Schema(args) = app.command else {
      panic!("expected to parse schema command");
    };
    assert_eq!(args.schema, Some(PathBuf::from("schema.graphql")));
    assert!(args.graphql_url.is_none());
    assert!(args.headers.is_empty());
    assert_eq!(args.timeout, Duration::from_secs(60));
  }

  #[test]
  fn schema_parses_graphql_url_headers_and_timeout() {
    let app = MockQLCli::try_parse_from([
      "mockql",
      "schema",
      "--graphql-url",
      "https://example.com/graphql",
      "--header",
      "client-info:test",
      "--timeout",
      "30s",
    ])
    .unwrap();

    let Commands::Schema(args) = app.command else {
      panic!("expected to parse schema command");
    };
    assert!(args.schema.is_none());
    assert_eq!(args.graphql_url.unwrap().as_str(), "https://example.com/graphql");
    assert_eq!(args.headers.len(), 1);
    assert_eq!(args.headers[0].0, HeaderName::from_static("client-info"));
    assert_eq!(args.headers[0].1, HeaderValue::from_static("test"));
    assert_eq!(args.timeout, Duration::from_secs(30));
  }
}
