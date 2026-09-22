//! Bounded captured execution for menu adapters.

use std::{
    fs::File,
    io::{self, Read, Seek},
    process::{Command, Output, Stdio},
    thread,
    time::{Duration, Instant},
};

const MAX_OUTPUT: u64 = 8 * 1024 * 1024;

pub fn capture(command: &mut Command) -> io::Result<Output> {
    capture_with_timeout(command, Duration::from_mins(1))
}

fn capture_with_timeout(command: &mut Command, timeout: Duration) -> io::Result<Output> {
    let mut stdout = tempfile::tempfile()?;
    let mut stderr = tempfile::tempfile()?;
    command
        .stdin(Stdio::null())
        .stdout(stdout.try_clone()?)
        .stderr(stderr.try_clone()?);
    hide_window(command);
    let mut child = command.spawn()?;
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
