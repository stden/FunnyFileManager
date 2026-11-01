//! Purpose: Waiting for a WSL process and its two output pipes together,
//! within the command's deadline and a short drain after it exits.

use std::sync::mpsc::TryRecvError;
use std::thread;
use std::time::{Duration, Instant};

use super::{Captured, Exit, Pipe, StreamOutput, DRAIN_GRACE, POLL_INTERVAL};

enum PipeProgress {
    Pending,
    Read,
    Closed,
    ClosedAfterRead,
}

impl PipeProgress {
    fn read(&self) -> bool {
        matches!(self, Self::Read | Self::ClosedAfterRead)
    }

    fn closed(&self) -> bool {
        matches!(self, Self::Closed | Self::ClosedAfterRead)
    }
}

fn drain_pipe(pipe: &Pipe, output: &mut StreamOutput) -> Result<PipeProgress, String> {
    let mut progress = PipeProgress::Pending;
    // A continuous writer must not prevent timeout and child-status checks.
    for _ in 0..4 {
        match pipe.try_recv() {
            Ok(bytes) => {
                output.append(&bytes?);
                progress = PipeProgress::Read;
            }
            Err(TryRecvError::Empty) => return Ok(progress),
            Err(TryRecvError::Disconnected) => {
                return Ok(if progress.read() {
                    PipeProgress::ClosedAfterRead
                } else {
                    PipeProgress::Closed
                });
            }
        }
    }
    Ok(progress)
}

pub(super) trait ManagedProcess {
    fn poll(&mut self) -> Result<Option<Exit>, String>;
}

pub(super) fn capture(
    child: &mut impl ManagedProcess,
    stdout_pipe: &Pipe,
    stderr_pipe: &Pipe,
    timeout: Duration,
) -> Result<Captured, String> {
    let deadline = Instant::now() + timeout;
    let mut drain_deadline = None;
    let mut exit = None;
    let mut stdout = StreamOutput::default();
    let mut stderr = StreamOutput::default();
    loop {
        let out_progress = drain_pipe(stdout_pipe, &mut stdout)?;
        let err_progress = drain_pipe(stderr_pipe, &mut stderr)?;
        poll_exit(child, &mut exit, &mut drain_deadline)?;
        if out_progress.closed() && err_progress.closed() {
            if let Some(exit) = exit {
                return Ok(Captured {
                    exit,
                    stdout,
                    stderr,
                });
            }
        }
        wait_for_output(
            deadline,
            timeout,
            &mut drain_deadline,
            exit.is_some(),
            out_progress.read() || err_progress.read(),
        )?;
    }
}

fn wait_for_output(
    deadline: Instant,
    timeout: Duration,
    drain_deadline: &mut Option<Instant>,
    exited: bool,
    read_output: bool,
) -> Result<(), String> {
    let now = Instant::now();
    if exited && read_output {
        // Bounded queues can keep delivering output after the child exits.
        // Only an idle inherited pipe consumes the drain grace; the total
        // command deadline still applies to continuously arriving output.
        *drain_deadline = Some(now + DRAIN_GRACE);
    }
    if now >= deadline {
        return Err(format!(
            "WSL command timed out after {} seconds",
            timeout.as_secs()
        ));
    }
    if drain_deadline.is_some_and(|deadline| now >= deadline) {
        return Err("WSL output did not close after the command exited".to_owned());
    }
    if !read_output {
        thread::sleep(POLL_INTERVAL.min(deadline.saturating_duration_since(now)));
    }
    Ok(())
}

fn poll_exit(
    child: &mut impl ManagedProcess,
    exit: &mut Option<Exit>,
    drain_deadline: &mut Option<Instant>,
) -> Result<(), String> {
    if exit.is_none() {
        *exit = child.poll()?;
        if exit.is_some() {
            *drain_deadline = Some(Instant::now() + DRAIN_GRACE);
        }
    }
    Ok(())
}
