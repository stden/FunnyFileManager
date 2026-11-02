//! Windows Subsystem for Linux adapter. Operations and state live in their own layers.

use crate::models::{WslOperation, WslOutput};

#[cfg(any(windows, test))]
mod encoding;
#[cfg(any(windows, test))]
mod output;
#[cfg(any(windows, test))]
mod process;

#[cfg(any(windows, test))]
use output::{check_exit, decode_output};

/// Executes WSL effects without coupling application operations to a host process.
pub trait WslBackend: Send + Sync {
    fn execute(&self, operation: &WslOperation) -> Result<WslOutput, String>;
}

/// Uses the Windows installation's `wsl.exe`, without shell or PATH lookup.
#[derive(Debug, Default)]
pub struct SystemWslBackend;

impl WslBackend for SystemWslBackend {
    fn execute(&self, operation: &WslOperation) -> Result<WslOutput, String> {
        #[cfg(windows)]
        {
            execute_with(operation, process::run)
        }
        #[cfg(not(windows))]
        {
            let _ = operation;
            Err("WSL is only available on Windows".to_owned())
        }
    }
}

// Distribution names are passed as a separate argument, never inserted into this script.
// These are ordinary installed-package upgrades, not distribution-release upgrades.
#[cfg(any(windows, test))]
const PACKAGE_UPDATE_SCRIPT: &str = include_str!("wsl/package_update.sh");

/// An operation that runs `wsl.exe`: every one but opening a root, which
/// needs no command, so nothing downstream has an `Open` to rule out.
#[cfg(any(windows, test))]
#[derive(Clone, Copy)]
enum WslCommand<'a> {
    Refresh,
    Update,
    UpdatePackages(&'a crate::models::DistributionName),
}

#[cfg(any(windows, test))]
fn execute_with(
    operation: &WslOperation,
    runner: impl FnOnce(&[&str], std::time::Duration) -> Result<process::Captured, String>,
) -> Result<WslOutput, String> {
    use std::time::Duration;

    let (command, captured) = match operation {
        WslOperation::Open(distribution) => return Ok(WslOutput::Root(distribution.root_path())),
        WslOperation::Refresh => (
            WslCommand::Refresh,
            runner(&["--list", "--quiet"], Duration::from_secs(20))?,
        ),
        WslOperation::Update => (
            WslCommand::Update,
            runner(&["--update"], Duration::from_mins(5))?,
        ),
        WslOperation::UpdatePackages(distribution) => (
            WslCommand::UpdatePackages(distribution),
            runner(
                &[
                    "--distribution",
                    distribution.as_str(),
                    "--user",
                    "root",
                    "--exec",
                    "sh",
                    "-lc",
                    PACKAGE_UPDATE_SCRIPT,
                ],
                Duration::from_hours(1),
            )?,
        ),
    };
    check_exit(command, &captured)?;
    decode_output(command, &captured)
}

#[cfg(test)]
mod tests;
