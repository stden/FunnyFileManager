//! Purpose: Typed shell commands: run one in the active panel's folder or
//! start it in a new console, record it in the transcript, and sum it up in
//! the status bar.

use std::{io, path::Path};

use crate::command::ShellCommand;
use crate::models::App;
use crate::ops::shell::{run_shell_command_with_timeout, ShellError};
use crate::status::Status;
use crate::utils::refresh_panels;

pub(super) fn cmd_new_console(
    app: &mut App,
    command: &ShellCommand,
    spawn: impl FnOnce(&ShellCommand, &Path) -> io::Result<()>,
) -> io::Result<()> {
    let folder = app.get_active_panel_ref().path.clone();
    spawn(command, &folder).map_err(|error| {
        let error = ShellError::launching_in(&folder, error);
        command_launch_error(app, command.as_str(), &error)
    })?;
    let status = Status::ConsoleStarted(command.clone());
    app.console.record_output(&status.to_string());
    app.set_status(status);
    Ok(())
}

fn command_launch_error(app: &mut App, command: &str, error: &ShellError) -> io::Error {
    let message = error.report(command);
    app.console.record_error(&message);
    io::Error::other(message)
}

/// Run `command` in the active panel's directory and report what it printed.
///
/// Everything the command wrote goes to the console transcript behind the
/// panels (`Ctrl+O` reveals it), the way Far Manager prints to the console
/// screen. The status bar keeps its one-line summary, so the common case —
/// glance at the result, keep working — costs no keystrokes at all.
pub(super) fn cmd_shell(app: &mut App, command: &ShellCommand) -> io::Result<()> {
    // No emptiness check: `ShellCommand` cannot be constructed from blank text,
    // so "run nothing" is not a state this function can be called in.
    let command = command.as_str();
    let panel_path = app.get_active_panel().path.clone();
    let seconds = app
        .settings
        .number(crate::menu::SettingsPage::PluginManager);
    let timeout = std::time::Duration::from_secs(u64::try_from(seconds).unwrap_or(u64::MAX));
    match run_shell_command_with_timeout(command, &panel_path, timeout) {
        Ok(output) => record_shell_output(app, command, &output),
        Err(ShellError::Cancelled) => record_cancel(app, command),
        // A shell that would not start is still part of the transcript: the
        // echoed command with nothing under it reads as "it worked silently".
        Err(error) => return Err(command_launch_error(app, command, &error)),
    }

    // The command may have created or removed files; a refresh failure is not
    // the command's failure, so it is deliberately not propagated.
    drop(refresh_panels(app));
    Ok(())
}

fn record_shell_output(app: &mut App, command: &str, output: &std::process::Output) {
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    app.console.record_output(&stdout);
    app.console.record_error(&stderr);
    app.set_status(shell_summary(&stdout, &stderr, output.status, command));
}

/// Esc / Ctrl+C stopped `command`. The user asked for that, so it is an
/// outcome to report, not an error.
pub(super) fn record_cancel(app: &mut App, command: &str) {
    let message = ShellError::Cancelled.report(command);
    app.console.record_error(&message);
    app.set_status(Status::CommandOutput(message));
}

/// The one line the status bar shows for a finished command.
fn shell_summary(
    stdout: &str,
    stderr: &str,
    status: std::process::ExitStatus,
    command: &str,
) -> Status {
    // Through the same sanitiser the transcript uses: a `ls --color` summary
    // would otherwise read `[31mred[0m` in the status bar while the console
    // two keystrokes away shows `red`. A line that is nothing but escapes —
    // `tput sgr0` — has nothing to show, so it does not count as output.
    if let Some(last) = crate::console::first_drawable(stdout.lines().rev()) {
        return Status::CommandOutput(last);
    }
    if let Some(first) = crate::console::first_drawable(stderr.lines()) {
        return Status::CommandOutput(format!("Err: {first}"));
    }
    if status.success() {
        return Status::CommandOutput(format!("OK: {command}"));
    }
    Status::CommandOutput(format!("Exit {status}"))
}
