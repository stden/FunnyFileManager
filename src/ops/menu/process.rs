//! Bounded captured execution for menu adapters.

use std::{
    fs::File,
    io::{self, Read, Seek},
    path::Path,
    process::{Child, Command, Output, Stdio},
    thread,
    time::{Duration, Instant},
};

const MAX_OUTPUT: u64 = 8 * 1024 * 1024;

pub fn capture(command: &mut Command) -> io::Result<Output> {
    capture_with_timeout(command, Duration::from_mins(1), &std::env::temp_dir())
}

/// Run `command`, its two streams spooled to files in `spool`.
fn capture_with_timeout(
    command: &mut Command,
    timeout: Duration,
    spool: &Path,
) -> io::Result<Output> {
    let mut stdout = spool_file(spool)?;
    let mut stderr = spool_file(spool)?;
    command
        .stdin(Stdio::null())
        .stdout(stdout.try_clone()?)
        .stderr(stderr.try_clone()?);
    hide_window(command);
    let mut child = spawn(command)?;
    let deadline = Instant::now() + timeout;
    let status = loop {
        if let Some(status) = child.try_wait()? {
            break status;
        }
        if Instant::now() >= deadline
            || stdout.metadata()?.len() + stderr.metadata()?.len() > MAX_OUTPUT
        {
            let _ = child.kill();
            let _ = child.wait();
            return Err(io::Error::new(
                io::ErrorKind::TimedOut,
                "Command exceeded its time or output limit",
            ));
        }
        thread::sleep(Duration::from_millis(20));
    };
    Ok(Output {
        status,
        stdout: read_output(&mut stdout)?,
        stderr: read_output(&mut stderr)?,
    })
}

/// A file in `spool` to catch one stream in. The OS error names no path, so
/// the directory is named here: a missing or read-only temp directory must
/// not read as a program that failed to start.
fn spool_file(spool: &Path) -> io::Result<File> {
    tempfile::tempfile_in(spool).map_err(|error| {
        let message = format!(
            "cannot create a temporary file in {}: {error}",
            spool.display()
        );
        io::Error::new(error.kind(), message)
    })
}

/// Start `command`; a failure names the program, which the OS error leaves
/// out.
fn spawn(command: &mut Command) -> io::Result<Child> {
    command.spawn().map_err(|error| {
        let program = command.get_program().to_string_lossy();
        io::Error::new(error.kind(), format!("cannot run {program}: {error}"))
    })
}

fn read_output(file: &mut File) -> io::Result<Vec<u8>> {
    if file.metadata()?.len() > MAX_OUTPUT {
        return Err(io::Error::other("Command output exceeds 8 MiB"));
    }
    file.rewind()?;
    let mut bytes = Vec::new();
    file.take(MAX_OUTPUT).read_to_end(&mut bytes)?;
    Ok(bytes)
}

#[cfg(windows)]
fn hide_window(command: &mut Command) {
    use std::os::windows::process::CommandExt;
    command.creation_flags(0x0800_0000);
}

#[cfg(not(windows))]
fn hide_window(_command: &mut Command) {}

#[cfg(test)]
mod tests;
