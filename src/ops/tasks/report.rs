//! Purpose: The task list where FFM has no process reader yet — Windows,
//! macOS and the BSDs until their phases land: the `ps` or `tasklist` report
//! in the F3 viewer, exactly as before the live viewer existed.

use std::io;
use std::process::Command;

use crate::models::{App, Overlay, Viewer};

/// Runs the report and shows its output; a report that cannot run, or that
/// fails, is the error the caller shows.
pub(super) fn open(app: &mut App) -> io::Result<()> {
    show(app, &mut report_command())
}

/// Runs `command` and shows what it printed as the Task list report.
fn show(app: &mut App, command: &mut Command) -> io::Result<()> {
    let output = crate::ops::menu::process::capture(command)?;
    if !output.status.success() {
        return Err(io::Error::other(
            String::from_utf8_lossy(&output.stderr).into_owned(),
        ));
    }
    let lines = String::from_utf8_lossy(&output.stdout)
        .lines()
        .map(str::to_owned)
        .collect();
    app.set_overlay(Overlay::Viewer(Viewer::report("Task list", lines)));
    Ok(())
}

/// Windows lists its processes with `tasklist`.
#[cfg(windows)]
fn report_command() -> Command {
    Command::new("tasklist")
}

/// Unix lists them with `ps`: PID, name, and ps's lifetime CPU and memory
/// shares.
#[cfg(not(windows))]
fn report_command() -> Command {
    let mut command = Command::new("ps");
    command.args(["-eo", "pid,comm,%cpu,%mem"]);
    command
}

#[cfg(test)]
mod tests;
#[cfg(all(test, unix))]
mod tests_unix;
