//! Purpose: Execute shell syntax in private PTY I/O without protocol handles.

use super::ShellError;
use crate::terminal::headless;
use std::{
    io,
    path::Path,
    process::{Command, ExitStatus, Output},
    time::Duration,
};

pub(super) fn run(
    command: &str,
    directory: &Path,
    timeout: Option<Duration>,
) -> Result<Output, ShellError> {
    #[cfg(windows)]
    let mut shell = shell(command).map_err(ShellError::Launch)?;
    #[cfg(not(windows))]
    let mut shell = shell(command);
    let result = headless::run_with_timeout(shell.command.current_dir(directory), timeout)
        .map_err(|error| failure(directory, error, timeout))?;
    Ok(Output {
        status: status(result.code),
        stdout: result.output,
        // A PTY presents the same merged output stream an interactive terminal does.
        stderr: Vec::new(),
    })
}

struct Shell {
    command: Command,
    #[cfg(windows)]
    _script: tempfile::NamedTempFile,
}

#[cfg(windows)]
fn shell(command: &str) -> io::Result<Shell> {
    use std::io::Write;
    let mut script = tempfile::Builder::new()
        .prefix("ffm-command-")
        .suffix(".cmd")
        .tempfile()?;
    // Keep shell quoting verbatim instead of applying C-runtime escaping to it.
    write!(script, "@echo off\r\n{command}\r\n")?;
    script.flush()?;
    let mut command = Command::new("cmd.exe");
    command.args(["/D", "/C"]).arg(script.path());
    Ok(Shell {
        command,
        _script: script,
    })
}

#[cfg(not(windows))]
fn shell(command: &str) -> Shell {
    let mut shell = Command::new("sh");
    shell.args(["-c", command]);
    Shell { command: shell }
}

fn failure(directory: &Path, error: io::Error, timeout: Option<Duration>) -> ShellError {
    match error.kind() {
        io::ErrorKind::Interrupted => ShellError::Cancelled,
        io::ErrorKind::TimedOut => ShellError::TimedOut(timeout.unwrap_or_default()),
        _ => ShellError::launching_in(directory, error),
    }
}

#[cfg(windows)]
fn status(code: u32) -> ExitStatus {
    use std::os::windows::process::ExitStatusExt;
    ExitStatus::from_raw(code)
}

#[cfg(not(windows))]
fn status(code: u32) -> ExitStatus {
    use std::os::unix::process::ExitStatusExt;
    ExitStatus::from_raw(i32::from(u8::try_from(code).unwrap_or(1)) << 8)
}

#[cfg(test)]
mod tests;
