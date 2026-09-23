//! Purpose: Run installers with a deadline, cancellation, bounded output, and owned process trees.

use super::{check_cancel, InstallError};
use std::{
    io::{Read, Seek, SeekFrom},
    process::{Child, Command, Stdio},
    sync::atomic::AtomicBool,
    time::{Duration, Instant},
};

const TIMEOUT: Duration = Duration::from_secs(600);

struct ChildTree {
    child: Option<Child>,
    #[cfg(windows)]
    job: Option<std::os::windows::io::OwnedHandle>,
}

impl ChildTree {
    fn start(command: &mut Command) -> Result<Self, InstallError> {
        crate::lsp::process::configure(command);
        let tree = Self {
            child: Some(command.spawn()?),
            #[cfg(windows)]
            job: None,
        };
        tree.attach()
    }

    #[cfg(windows)]
    fn attach(mut self) -> Result<Self, InstallError> {
        // `start` has just spawned the child, so it is always there.
        let child = self.child.as_ref().ok_or(InstallError::Worker)?;
        self.job = Some(crate::windows::process_tree_job(child)?);
        Ok(self)
    }

    #[cfg(unix)]
    #[expect(
        clippy::unnecessary_wraps,
        reason = "shares the Windows signature, where assigning the job object can fail"
    )]
    fn attach(self) -> Result<Self, InstallError> {
        Ok(self)
    }

    fn exited(&mut self) -> bool {
        self.child.as_mut().is_none_or(crate::lsp::process::exited)
    }

    fn finish(&mut self) -> Result<(), InstallError> {
        // Only `finish` takes the child, and it runs once per tree.
        let mut child = self.child.take().ok_or(InstallError::Worker)?;
        crate::lsp::process::terminate(&mut child);
        if child.wait()?.success() {
            Ok(())
        } else {
            Err(InstallError::Command)
        }
    }
}

impl Drop for ChildTree {
    fn drop(&mut self) {
        #[cfg(windows)]
        self.job.take();
        if let Some(child) = &mut self.child {
            crate::lsp::process::terminate(child);
        }
    }
}

pub(super) fn run(command: &mut Command, stop: &AtomicBool) -> Result<Vec<u8>, InstallError> {
    run_until(command, stop, Instant::now() + TIMEOUT)
}

fn run_until(
    command: &mut Command,
    stop: &AtomicBool,
    deadline: Instant,
) -> Result<Vec<u8>, InstallError> {
    check_cancel(stop)?;
    let mut output = tempfile::tempfile()?;
    let mut errors = tempfile::tempfile()?;
    command
        .stdin(Stdio::null())
        .stdout(output.try_clone()?)
        .stderr(errors.try_clone()?);
    let mut process = ChildTree::start(command)?;
    wait_for(&mut process, &output, &mut errors, stop, deadline)?;
    // Terminate descendants before reaping the leader, retaining its PID on Unix.
    if let Err(error) = process.finish() {
        return Err(noted(error, &mut errors));
    }
    if output.metadata()?.len() > OUTPUT_LIMIT || errors.metadata()?.len() > OUTPUT_LIMIT {
        return Err(reject(&mut errors));
    }
    read_output(&mut output)
}

const OUTPUT_LIMIT: u64 = 1_048_576;

fn wait_for(
    process: &mut ChildTree,
    output: &std::fs::File,
    errors: &mut std::fs::File,
    stop: &AtomicBool,
    deadline: Instant,
) -> Result<(), InstallError> {
    while !process.exited() {
        check_cancel(stop)?;
        if Instant::now() >= deadline {
            return Err(InstallError::Timeout);
        }
        if output.metadata()?.len() > OUTPUT_LIMIT || errors.metadata()?.len() > OUTPUT_LIMIT {
            return Err(reject(errors));
        }
        std::thread::sleep(Duration::from_millis(20));
    }
    Ok(())
}

fn noted(error: InstallError, errors: &mut std::fs::File) -> InstallError {
    match error {
        InstallError::Command => reject(errors),
        other => other,
    }
}

fn reject(errors: &mut std::fs::File) -> InstallError {
    if let Some(line) = failure_line(errors) {
        crate::diagnostics::log_error(&format!("LSP installer: {line}"));
    }
    InstallError::Command
}

pub(super) fn failure_line(errors: &mut (impl Read + Seek)) -> Option<String> {
    errors.seek(SeekFrom::Start(0)).ok()?;
    let mut bytes = Vec::new();
    errors.take(4096).read_to_end(&mut bytes).ok()?;
    let text = String::from_utf8_lossy(&bytes);
    let line = text.lines().map(str::trim).find(|line| !line.is_empty())?;
    Some(line.chars().take(200).collect())
}

fn read_output(output: &mut std::fs::File) -> Result<Vec<u8>, InstallError> {
    output.seek(SeekFrom::Start(0))?;
    let mut bytes = Vec::new();
    output.take(65_536).read_to_end(&mut bytes)?;
    Ok(bytes)
}

#[cfg(test)]
mod tests;
