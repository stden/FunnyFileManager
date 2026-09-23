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
        let mut tree = Self {
            child: Some(command.spawn()?),
            #[cfg(windows)]
            job: None,
        };
        tree.attach()?;
        Ok(tree)
    }

    #[cfg(windows)]
    fn attach(&mut self) -> Result<(), InstallError> {
        self.job = Some(crate::windows::lsp_job(
            self.child.as_ref().expect("spawn owns the child"),
        )?);
        Ok(())
    }

    #[cfg(unix)]
    fn attach(&mut self) -> Result<(), InstallError> {
        Ok(())
    }

    fn exited(&mut self) -> bool {
        self.child.as_mut().is_none_or(crate::lsp::process::exited)
    }

    fn finish(&mut self) -> Result<(), InstallError> {
        let mut child = self.child.take().expect("finish consumes the child once");
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
    command
        .stdin(Stdio::null())
        .stdout(output.try_clone()?)
        .stderr(Stdio::null());
    let mut process = ChildTree::start(command)?;
    while !process.exited() {
        check_cancel(stop)?;
        if Instant::now() >= deadline {
            return Err(InstallError::Timeout);
        }
        if output.metadata()?.len() > 1_048_576 {
            return Err(InstallError::Command);
        }
        std::thread::sleep(Duration::from_millis(20));
    }
    // Terminate descendants before reaping the leader, retaining its PID on Unix.
    process.finish()?;
    output.seek(SeekFrom::Start(0))?;
    let mut bytes = Vec::new();
    output.take(65_536).read_to_end(&mut bytes)?;
    Ok(bytes)
}

#[cfg(test)]
mod tests;
