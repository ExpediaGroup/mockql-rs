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
