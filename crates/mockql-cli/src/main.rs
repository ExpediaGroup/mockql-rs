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

//! Command-line entrypoint for executing mock-aware GraphQL operations.
#![deny(missing_docs)]

#[path = "commands/mod.rs"]
mod commands;

use crate::commands::OneshotArgs;
use crate::commands::ProxyArgs;
use crate::commands::SchemaArgs;
use crate::commands::exec_oneshot;
use crate::commands::exec_proxy;
use crate::commands::exec_schema;
use clap::Parser;
use clap::Subcommand;
use mockql_core::SchemaFilterError;
use mockql_core::SchemaLoadError;
use mockql_core::ServiceError;
use std::process::ExitCode;
use thiserror::Error;

#[derive(Debug, Parser)]
#[command(name = "mockql", about = "AI Generated GraphQL response mocking via @mock")]
struct MockQLCli {
  #[command(subcommand)]
  command: Commands,
}

#[derive(Debug, Subcommand)]
enum Commands {
  #[command(about = "Execute a single mock GraphQL request and print the result to stdout")]
  Oneshot(OneshotArgs),
  #[command(about = "Start a HTTP proxy server that mocks GraphQL responses")]
  Proxy(ProxyArgs),
  #[command(about = "Print a GraphQL schema decorated with the mock directive")]
  Schema(SchemaArgs),
}

#[derive(Debug, Error)]
pub(crate) enum CliError {
  #[error("failed to read file: {0}")]
  Io(#[from] std::io::Error),
  #[error("failed to parse JSON: {0}")]
  Json(#[from] serde_json::Error),
  #[error("{0}")]
  Message(String),
  #[error(transparent)]
  Schema(#[from] SchemaLoadError),
  #[error(transparent)]
  SchemaFilter(#[from] SchemaFilterError),
  #[error("failed to build HTTP client: {0}")]
  ClientInit(String),
  #[error(transparent)]
  Service(#[from] ServiceError),
}

#[tokio::main]
async fn main() -> ExitCode {
  let result = match MockQLCli::parse().command {
    Commands::Oneshot(args) => exec_oneshot(args).await,
    Commands::Proxy(args) => exec_proxy(args).await,
    Commands::Schema(args) => exec_schema(args).await,
  };
  match result {
    Ok(()) => ExitCode::SUCCESS,
    Err(error) => {
      eprintln!("{error}");
      ExitCode::FAILURE
    }
  }
}
