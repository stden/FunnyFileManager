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
pub fn capture_with_timeout(
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
    detach(command);
    let mut child = spawn(command)?;
    let tree = Tree::of(&child);
    let deadline = Instant::now() + timeout;
    let status = loop {
        if let Some(status) = child.try_wait()? {
            break status;
        }
        if Instant::now() >= deadline
            || stdout.metadata()?.len() + stderr.metadata()?.len() > MAX_OUTPUT
        {
            return Err(stop(&mut child, tree));
        }
        thread::sleep(Duration::from_millis(20));
    };
    Ok(Output {
        status,
        stdout: read_output(&mut stdout)?,
        stderr: read_output(&mut stderr)?,
    })
}

/// Kill a child that ran past its limits, with everything it started, and
/// say so. A kill or a reap that fails is added to the message rather than
/// dropped: the child may still be running, and the user should hear it.
fn stop(child: &mut Child, tree: Tree) -> io::Error {
    let problems = [
        failed("stopping it", tree.kill(child)),
        failed("waiting for it", child.wait().map(drop)),
    ];
    let message: Vec<String> =
        std::iter::once("Command exceeded its time or output limit".to_owned())
            .chain(problems.into_iter().flatten())
            .collect();
    io::Error::new(io::ErrorKind::TimedOut, message.join("; "))
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

/// Everything the child starts, as one thing to stop. A shim stops only
/// itself on a plain kill — npm's `claude.cmd` is a `cmd.exe`, Volta's is a
/// program of its own — while the real CLI under it runs on and may write its
/// config after the report said it did not finish.
///
/// Unix: the child leads a process group of its own (see [`detach`]).
#[cfg(unix)]
struct Tree;

/// Windows: a job that kills every process in it when its handle closes;
/// `None` when the job could not be made, and then a kill stops the child
/// alone, as it always did.
#[cfg(windows)]
struct Tree(Option<std::os::windows::io::OwnedHandle>);

impl Tree {
    #[cfg(unix)]
    const fn of(_child: &Child) -> Self {
        Self
    }

    #[cfg(windows)]
    fn of(child: &Child) -> Self {
        Self(crate::windows::process_tree_job(child).ok())
    }

    /// Kill the child and everything in its tree.
    #[cfg(unix)]
    #[expect(
        clippy::unused_self,
        reason = "the Windows tree holds a job; Unix finds the group from the child"
    )]
    fn kill(self, child: &mut Child) -> io::Result<()> {
        // No group left means nothing but the child itself to stop.
        crate::platform::kill_process_group(child).or_else(|_| child.kill())
    }

    #[cfg(windows)]
    fn kill(self, child: &mut Child) -> io::Result<()> {
        // Closing the job kills every process in it; then the child itself,
        // in case there was no job.
        drop(self.0);
        child.kill()
    }
}

/// Start the child with no console window of its own.
#[cfg(windows)]
fn detach(command: &mut Command) {
    use std::os::windows::process::CommandExt;
    command.creation_flags(0x0800_0000);
}

/// Start the child as the leader of a process group of its own, so a stop
/// reaches what it starts. Nothing it runs reads the terminal: stdin is
/// empty.
#[cfg(unix)]
fn detach(command: &mut Command) {
    use std::os::unix::process::CommandExt;
    command.process_group(0);
}

#[cfg(test)]
mod tests;
