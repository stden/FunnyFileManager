//! Purpose: Capture association-launcher status without sharing protocol I/O.

use crate::terminal::headless;
use std::{io, path::Path, process::Command, time::Duration};

pub(super) fn open(path: &Path) -> io::Result<()> {
    open_with(path, |command| {
        headless::run_detached(command, Duration::from_secs(30))
    })
}

pub(super) fn edit(path: &Path) -> io::Result<()> {
    edit_with(path, |command| {
        headless::run_detached(command, Duration::from_secs(30))
    })
}

fn open_with(
    path: &Path,
    run: impl FnOnce(&mut Command) -> io::Result<headless::ExternalOutcome>,
) -> io::Result<()> {
    run_with(&mut command(path), run)
}

fn edit_with(
    path: &Path,
    run: impl FnOnce(&mut Command) -> io::Result<headless::ExternalOutcome>,
) -> io::Result<()> {
    run_with(&mut edit_command(path), run)
}

fn run_with(
    command: &mut Command,
    run: impl FnOnce(&mut Command) -> io::Result<headless::ExternalOutcome>,
) -> io::Result<()> {
    let outcome = run(command)?;
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
fn edit_command(path: &Path) -> Command {
    let literal = path.to_string_lossy().replace('\'', "''");
    let script = format!(
        "$ErrorActionPreference = 'Stop'; try {{ Start-Process -FilePath '{literal}' -Verb Edit }} catch {{ Start-Process -FilePath '{literal}' }}"
    );
    let mut command = Command::new("powershell.exe");
    command.args(["-NoProfile", "-NonInteractive", "-Command", &script]);
    command
}

#[cfg(not(windows))]
fn edit_command(path: &Path) -> Command {
    super::edit::unix_command(path)
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
    #[cfg(target_os = "macos")]
    const OPENER: &str = "open";
    #[cfg(not(target_os = "macos"))]
    const OPENER: &str = "xdg-open";
    let mut command = Command::new(OPENER);
    command.arg(path);
    command
}

#[cfg(test)]
mod tests;
