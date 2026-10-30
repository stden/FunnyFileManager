//! Purpose: Diagnostic reports: run a PowerShell script and show what it
//! printed in a viewer. Compiled on Windows, and in tests on every platform.

use crate::models::{App, Viewer};
use std::io;

#[cfg(windows)]
use std::process::Command;

pub(super) const SYSTEM_MESSAGES_TITLE: &str = "Windows System Messages";
pub(super) const PERFORMANCE_TITLE: &str = "Windows Performance";

#[cfg(windows)]
const CREATE_NO_WINDOW: u32 = 0x0800_0000;

#[cfg(windows)]
pub(super) const SYSTEM_MESSAGES_SCRIPT: &str = include_str!("system_messages.ps1");

#[cfg(windows)]
pub(super) const PERFORMANCE_SCRIPT: &str = include_str!("performance.ps1");

pub(super) fn report_lines(report: &str) -> Vec<String> {
    let mut lines: Vec<String> = report
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(ToOwned::to_owned)
        .collect();

    if lines.is_empty() {
        lines.push("<no data>".to_owned());
    }

    lines
}

pub(super) fn open_report_viewer(app: &mut App, title: &str, report: &str) {
    app.set_overlay(crate::models::Overlay::Viewer(Viewer::report(
        title,
        report_lines(report),
    )));
    app.set_status(crate::status::Status::OpenedView(title.to_owned()));
}

pub(super) fn open_report_with<F>(app: &mut App, title: &str, runner: F) -> io::Result<()>
where
    F: FnOnce() -> io::Result<String>,
{
    let report = runner()?;
    open_report_viewer(app, title, &report);
    Ok(())
}

pub(super) const fn powershell_executable() -> &'static str {
    "powershell.exe"
}

pub(super) const fn powershell_report_args(script: &str) -> [&str; 7] {
    [
        "-NoLogo",
        "-NoProfile",
        "-NonInteractive",
        "-ExecutionPolicy",
        "Bypass",
        "-Command",
        script,
    ]
}

#[cfg(windows)]
pub(super) fn run_powershell_report(script: &str) -> io::Result<String> {
    use std::os::windows::process::CommandExt;

    let mut command = Command::new(powershell_executable());
    command.creation_flags(CREATE_NO_WINDOW);
    let output = command.args(powershell_report_args(script)).output()?;

    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).trim().to_owned();

    if output.status.success() {
        Ok(stdout)
    } else if !stderr.is_empty() {
        Err(io::Error::other(stderr))
    } else {
        Err(io::Error::other(format!(
            "PowerShell exited with status {}",
            output.status
        )))
    }
}

/// Classify a command's output into Ok(stdout) or an appropriate error.
/// Extracted so mutation tests can exercise the stderr-vs-status branches
/// on every platform.
#[cfg(test)]
pub(super) fn classify_command_output(
    success: bool,
    stdout: &str,
    stderr: &str,
) -> io::Result<String> {
    if success {
        Ok(stdout.to_owned())
    } else if !stderr.is_empty() {
        Err(io::Error::other(stderr.to_owned()))
    } else {
        Err(io::Error::other(
            "Command exited with failure status".to_owned(),
        ))
    }
}
