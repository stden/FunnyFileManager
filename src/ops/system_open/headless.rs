//! Purpose: Capture association-launcher status without sharing protocol I/O.

use crate::terminal::headless;
use std::{io, path::Path, process::Command, time::Duration};

pub(super) fn open(path: &Path) -> io::Result<()> {
    open_with(path, |command| {
        headless::run_detached(command, Duration::from_secs(30))
    })
}

fn open_with(
    path: &Path,
    run: impl FnOnce(&mut Command) -> io::Result<headless::ExternalOutcome>,
) -> io::Result<()> {
    let outcome = run(&mut command(path))?;
    if outcome.code == 0 {
        Ok(())
    } else {
        Err(io::Error::other(format!(
            "The association launcher exited with code {}",
            outcome.code
        )))
    }
}

#[cfg(windows)]
fn command(path: &Path) -> Command {
    let literal = path.to_string_lossy().replace('\'', "''");
    let script = format!("$ErrorActionPreference = 'Stop'; Start-Process -FilePath '{literal}'");
    let mut command = Command::new("powershell.exe");
    command.args(["-NoProfile", "-NonInteractive", "-Command", &script]);
    command
}

#[cfg(not(windows))]
fn command(path: &Path) -> Command {
    let mut command = Command::new(if cfg!(target_os = "macos") {
        "open"
    } else {
        "xdg-open"
    });
    command.arg(path);
    command
}

#[cfg(test)]
mod tests;
