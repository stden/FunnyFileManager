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

mod tree;
use tree::{detach, Tree};

pub fn capture(command: &mut Command) -> io::Result<Output> {
    capture_with_timeout(command, Duration::from_mins(1), &std::env::temp_dir())
}

/// Run `command`, its two streams spooled to files in `spool`.
pub fn capture_with_timeout(
    command: &mut Command,
    timeout: Duration,
    spool: &Path,
) -> io::Result<Output> {
    capture_cancellable(command, timeout, spool, || false)
}

/// Run with the same limits, and stop the process tree on cancellation.
pub(in crate::ops) fn capture_cancellable(
    command: &mut Command,
    timeout: Duration,
    spool: &Path,
    cancelled: impl FnMut() -> bool,
) -> io::Result<Output> {
    let mut stdout = spool_file(spool)?;
    let mut stderr = spool_file(spool)?;
    command
        .stdin(Stdio::null())
        .stdout(stdout.try_clone()?)
        .stderr(stderr.try_clone()?);
    detach(command);
    let mut child = spawn(command)?;
    let tree = Tree::of(&child);
    let (status, _tree) = wait(&mut child, tree, [&stdout, &stderr], timeout, cancelled)?;
    Ok(Output {
        status,
        stdout: read_output(&mut stdout)?,
        stderr: read_output(&mut stderr)?,
    })
}

fn wait(
    child: &mut Child,
    tree: Tree,
    files: [&File; 2],
    timeout: Duration,
    mut cancelled: impl FnMut() -> bool,
) -> io::Result<(std::process::ExitStatus, Tree)> {
    let deadline = Instant::now() + timeout;
    loop {
        if let Some(status) = child.try_wait()? {
            return Ok((status, tree));
        }
        if cancelled() {
            return Err(stop(child, tree, StopReason::Cancelled));
        }
        let size = files.iter().try_fold(0, |sum, file| {
            file.metadata().map(|metadata| sum + metadata.len())
        })?;
        if Instant::now() >= deadline || size > MAX_OUTPUT {
            return Err(stop(child, tree, StopReason::Limit));
        }
        thread::sleep(Duration::from_millis(20));
    }
}

#[derive(Clone, Copy)]
enum StopReason {
    Cancelled,
    Limit,
}

/// Kill a child that ran past its limits, with everything it started, and
/// say so. A kill or a reap that fails is added to the message rather than
/// dropped: the child may still be running, and the user should hear it.
fn stop(child: &mut Child, tree: Tree, reason: StopReason) -> io::Error {
    let (kind, message) = match reason {
        StopReason::Cancelled => (io::ErrorKind::Interrupted, "Command cancelled"),
        StopReason::Limit => (
            io::ErrorKind::TimedOut,
            "Command exceeded its time or output limit",
        ),
    };
    let problems = [
        failed("stopping it", tree.kill(child)),
        failed("waiting for it", child.wait().map(drop)),
    ];
    let message: Vec<String> = std::iter::once(message.to_owned())
        .chain(problems.into_iter().flatten())
        .collect();
    io::Error::new(kind, message.join("; "))
}

/// `<what> failed: <error>` when `result` is an error.
fn failed(what: &str, result: io::Result<()>) -> Option<String> {
    result.err().map(|error| format!("{what} failed: {error}"))
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

#[cfg(test)]
mod tests;
