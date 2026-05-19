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

//! Workspace automation tasks for formatting and linting.
#![deny(missing_docs)]

mod fmt;
mod lint;
mod release;
mod test;

use anyhow::Result;
use clap::Parser;
use clap::Subcommand;

#[derive(Debug, Parser)]
#[command(author, version, about = "Workspace automation tasks")]
struct XTask {
  #[command(subcommand)]
  command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
  /// Enforce formatting and run lints.
  Lint,
  /// Fix formatting.
  Fmt,
  /// Run tests.
  Test,
  /// Prepare a coordinated workspace release.
  Release {
    #[command(subcommand)]
    command: release::Command,
  },
}

fn main() -> Result<()> {
  XTask::parse().run()
}

impl XTask {
  fn run(self) -> Result<()> {
    match self.command {
      Command::Lint => lint::run(),
      Command::Fmt => fmt::run(),
      Command::Test => test::run(),
      Command::Release { command } => release::run(command),
    }
  }
}
