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

use super::super::CliError;
use clap::Args;
use reqwest::Client;
use std::env;
use std::path::PathBuf;
use std::process::Command;

const INSTALL_SCRIPT_URL: &str = "https://raw.githubusercontent.com/ExpediaGroup/mockql-rs/main/install";

#[derive(Debug, Clone, Args)]
pub(crate) struct UpgradeArgs {
  /// Target version to install (defaults to latest release), e.g. `v0.0.7` or `0.0.7`.
  #[arg(long)]
  pub(super) version: Option<String>,
}

pub(crate) async fn exec_upgrade(args: UpgradeArgs) -> Result<(), CliError> {
  // ponytail: delegates to the GitHub `install` script to keep target detection,
  // checksum verification, and extraction in one place; add a native updater if Windows support is needed.
  let script = Client::new()
    .get(INSTALL_SCRIPT_URL)
    .send()
    .await
    .and_then(|response| response.error_for_status())
    .map_err(|e| CliError::Message(format!("failed to download install script: {e}")))?
    .text()
    .await
    .map_err(|e| CliError::Message(format!("failed to read install script: {e}")))?;

  let mut cmd = Command::new("sh");
  cmd.arg("-c").arg(&script);

  if let Some(version) = args.version {
    cmd.env("MOCKQL_VERSION", version);
  }

  if env::var_os("MOCKQL_INSTALL_DIR").is_none()
    && let Some(dir) = current_exe_dir()
  {
    cmd.env("MOCKQL_INSTALL_DIR", dir);
  }

  run_shell(cmd)
}

fn current_exe_dir() -> Option<PathBuf> {
  let exe = env::current_exe().ok()?;
  let resolved = exe.canonicalize().unwrap_or(exe);
  resolved.parent().map(PathBuf::from)
}

#[cfg(unix)]
fn run_shell(mut cmd: Command) -> Result<(), CliError> {
  use std::os::unix::process::CommandExt;

  let err = cmd.exec();
  Err(CliError::Message(format!("failed to execute install script: {err}")))
}

#[cfg(not(unix))]
fn run_shell(mut cmd: Command) -> Result<(), CliError> {
  let status = cmd.status()?;
  if status.success() {
    Ok(())
  } else {
    Err(CliError::Message(format!("upgrade failed with exit status: {status}")))
  }
}

#[cfg(test)]
mod tests {
  use crate::Commands;
  use crate::MockQLCli;
  use clap::Parser;

  #[test]
  fn upgrade_parses_default_and_version() {
    let app = MockQLCli::try_parse_from(["mockql", "upgrade"]).unwrap();
    let Commands::Upgrade(args) = app.command else {
      panic!("expected upgrade command");
    };
    assert!(args.version.is_none());

    let app = MockQLCli::try_parse_from(["mockql", "upgrade", "--version", "v0.0.7"]).unwrap();
    let Commands::Upgrade(args) = app.command else {
      panic!("expected upgrade command");
    };
    assert_eq!(args.version.as_deref(), Some("v0.0.7"));
  }
}
