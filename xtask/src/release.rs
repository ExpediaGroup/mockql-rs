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

use anyhow::Result;
use anyhow::anyhow;
use anyhow::bail;
use clap::Subcommand;
use xshell::Shell;
use xshell::cmd;

macro_rules! replace_in_file {
  ($path:expr, $regex:expr, $replacement:expr) => {
    let before = std::fs::read_to_string($path).map_err(|err| anyhow!("failed to read {:?}: {}", $path, err))?;
    let re = regex::Regex::new(&format!("(?m){}", $regex))?;
    let after = re.replace_all(&before, $replacement);
    std::fs::write($path, after.as_ref()).map_err(|err| anyhow!("failed to write {:?}: {}", $path, err))?;
  };
}

/// Release automation commands.
#[derive(Debug, Subcommand)]
pub(crate) enum Command {
  /// Update crate versions and exact internal dependency requirements.
  Prepare {
    /// Release version, for example `0.2.0` or `v0.2.0`.
    version: String,
  },
}

/// Run a release command.
pub(crate) fn run(command: Command) -> Result<()> {
  match command {
    Command::Prepare { version } => prepare(&version),
  }
}

fn prepare(version: &str) -> Result<()> {
  let version = normalize_version(version)?;

  replace_in_file!(
    "Cargo.toml",
    r#"^version = "[^"]+""#,
    format!(r#"version = "{version}""#)
  );
  replace_in_file!(
    "crates/mockql-cli/Cargo.toml",
    r#"^mockql-core = \{ path = "../mockql-core"(?:, version = "=?[^"]+")? \}"#,
    format!(r#"mockql-core = {{ path = "../mockql-core", version = "={version}" }}"#)
  );

  let sh = Shell::new()?;
  cmd!(sh, "cargo check --workspace").run()?;

  Ok(())
}

fn validate_version(version: &str) -> Result<()> {
  let core_version = version.split_once('-').map_or(version, |(core, _)| core);
  let parts = core_version.split('.').collect::<Vec<_>>();
  if parts.len() != 3
    || parts
      .iter()
      .any(|part| part.is_empty() || !part.chars().all(|ch| ch.is_ascii_digit()))
  {
    bail!("release version must be semver-like: {version}");
  }

  Ok(())
}

fn normalize_version(version: &str) -> Result<&str> {
  let version = version.strip_prefix('v').unwrap_or(version);
  validate_version(version)?;
  Ok(version)
}

#[cfg(test)]
mod tests {
  use super::normalize_version;

  #[test]
  fn normalize_version_should_accept_plain_semver() {
    assert_eq!(normalize_version("0.2.0").unwrap(), "0.2.0");
  }

  #[test]
  fn normalize_version_should_strip_v_prefix() {
    assert_eq!(normalize_version("v0.2.0").unwrap(), "0.2.0");
  }

  #[test]
  fn normalize_version_should_reject_invalid_version() {
    assert!(normalize_version("v0.2").is_err());
  }
}
