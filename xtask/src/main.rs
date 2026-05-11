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

use anyhow::Result;
use clap::Parser;
use clap::Subcommand;
use xshell::Shell;
use xshell::cmd;

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
}

fn main() -> Result<()> {
  XTask::parse().run()
}

impl XTask {
  fn run(self) -> Result<()> {
    match self.command {
      Command::Lint => exec_lint(),
      Command::Fmt => exec_fmt(),
      Command::Test => exec_test(),
    }
  }
}

fn exec_lint() -> Result<()> {
  let sh = Shell::new()?;
  cmd!(sh, "cargo fmt --all -- --check").run()?;
  cmd!(
    sh,
    "cargo clippy --workspace --all-targets --all-features -- -D warnings"
  )
  .run()?;
  Ok(())
}

fn exec_fmt() -> Result<()> {
  let sh = Shell::new()?;
  cmd!(sh, "cargo fmt --all").run()?;
  Ok(())
}

fn exec_test() -> Result<()> {
  let sh = Shell::new()?;
  cmd!(sh, "cargo test --workspace --all-features").run()?;
  Ok(())
}
