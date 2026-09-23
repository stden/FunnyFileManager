//! Bounded process lifetime and output capture for the WSL adapter.

use std::io::Read;
use std::sync::mpsc::{self, Receiver, TryRecvError};
use std::thread;
use std::time::{Duration, Instant};

const OUTPUT_LIMIT: usize = 256 * 1024;
const PIPE_CHUNK: usize = 8192;
const POLL_INTERVAL: Duration = Duration::from_millis(20);
const DRAIN_GRACE: Duration = Duration::from_millis(500);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Exit {
    Success,
    Failed(Option<i32>),
}

#[derive(Debug, Default)]
pub(super) struct StreamOutput {
    pub bytes: Vec<u8>,
    pub truncated: bool,
}

impl StreamOutput {
    fn append(&mut self, bytes: &[u8]) {
        let retained = bytes.len().min(OUTPUT_LIMIT - self.bytes.len());
        self.bytes.extend_from_slice(&bytes[..retained]);
        self.truncated |= retained < bytes.len();
    }
}

#[derive(Debug)]
pub(super) struct Captured {
    pub exit: Exit,
    pub stdout: StreamOutput,
    pub stderr: StreamOutput,
}

type Pipe = Receiver<Result<Vec<u8>, String>>;

fn read_pipe(reader: impl Read + Send + 'static) -> Result<Pipe, String> {
    // The queue is bounded as well as the retained output, even for verbose upgrades.
    let (sender, receiver) = mpsc::sync_channel(4);
    thread::Builder::new()
        .name("wsl-output".to_owned())
        .spawn(move || pump_pipe(reader, sender))
        .map_err(|error| format!("Cannot capture WSL output: {error}"))?;
    Ok(receiver)
}

/// Drain one stream into the bounded queue until EOF or its consumer closes.
fn pump_pipe(mut reader: impl Read, sender: mpsc::SyncSender<Result<Vec<u8>, String>>) {
    let mut buffer = [0; PIPE_CHUNK];
    loop {
        let bytes = match reader.read(&mut buffer) {
            Ok(0) => break,
            Ok(length) => buffer[..length].to_vec(),
            Err(error) if error.kind() == std::io::ErrorKind::Interrupted => continue,
            Err(error) => {
                let _ = sender.send(Err(format!("Cannot read WSL output: {error}")));
                break;
            }
        };
        if sender.send(Ok(bytes)).is_err() {
            break;
        }
    }
}

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

trait ManagedProcess {
    fn poll(&mut self) -> Result<Option<Exit>, String>;
}

fn capture(
    child: &mut impl ManagedProcess,
    stdout_pipe: Pipe,
    stderr_pipe: Pipe,
    timeout: Duration,
) -> Result<Captured, String> {
    let deadline = Instant::now() + timeout;
    let mut drain_deadline = None;
    let mut exit = None;
    let mut stdout = StreamOutput::default();
    let mut stderr = StreamOutput::default();
    loop {
        let out_progress = drain_pipe(&stdout_pipe, &mut stdout)?;
        let err_progress = drain_pipe(&stderr_pipe, &mut stderr)?;
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

#[cfg(windows)]
struct ChildGuard(std::process::Child);

#[cfg(windows)]
impl Drop for ChildGuard {
    fn drop(&mut self) {
        // Terminate on setup failure or timeout, without an unbounded wait if
        // Windows cannot stop the client. Linux descendants may still continue.
        if matches!(self.0.try_wait(), Ok(Some(_))) {
            return;
        }
        let _ = self.0.kill();
        let deadline = Instant::now() + DRAIN_GRACE;
        while matches!(self.0.try_wait(), Ok(None)) && Instant::now() < deadline {
            thread::sleep(POLL_INTERVAL);
        }
    }
}

#[cfg(windows)]
impl ManagedProcess for ChildGuard {
    fn poll(&mut self) -> Result<Option<Exit>, String> {
        let Some(status) = self
            .0
            .try_wait()
            .map_err(|error| format!("Cannot wait for WSL: {error}"))?
        else {
            return Ok(None);
        };
        Ok(Some(if status.success() {
            Exit::Success
        } else {
            Exit::Failed(status.code())
        }))
    }
}

#[cfg(windows)]
#[allow(unsafe_code)]
fn windows_directory() -> Result<std::path::PathBuf, String> {
    use std::os::windows::ffi::OsStringExt;
    use windows_sys::Win32::System::SystemInformation::GetWindowsDirectoryW;

    let mut buffer = vec![0; 260];
    loop {
        // SAFETY: the buffer is writable for the exact capacity passed to Win32.
        let length = unsafe { GetWindowsDirectoryW(buffer.as_mut_ptr(), buffer.len() as u32) };
        if length == 0 {
            return Err(format!(
                "Cannot locate Windows: {}",
                std::io::Error::last_os_error()
            ));
        }
        if (length as usize) < buffer.len() {
            return Ok(std::ffi::OsString::from_wide(&buffer[..length as usize]).into());
        }
        if length > 32_768 {
            return Err("Windows installation path is too long".to_owned());
        }
        buffer.resize(length as usize + 1, 0);
    }
}

#[cfg(windows)]
fn command(windows_directory: &std::path::Path, args: &[&str]) -> std::process::Command {
    use std::os::windows::process::CommandExt;
    use std::process::{Command, Stdio};

    let system_directory = if cfg!(target_pointer_width = "32") {
        "Sysnative"
    } else {
        "System32"
    };
    let mut command = Command::new(windows_directory.join(system_directory).join("wsl.exe"));
    command
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .creation_flags(0x0800_0000); // CREATE_NO_WINDOW
    command
}

#[cfg(windows)]
pub(super) fn run(args: &[&str], timeout: Duration) -> Result<Captured, String> {
    let mut child = ChildGuard(
        command(&windows_directory()?, args)
            .spawn()
            .map_err(|error| format!("Cannot start WSL; ensure it is installed: {error}"))?,
    );
    let stdout = read_pipe(child.0.stdout.take().ok_or("WSL stdout pipe is missing")?)?;
    let stderr = read_pipe(child.0.stderr.take().ok_or("WSL stderr pipe is missing")?)?;
    capture(&mut child, stdout, stderr, timeout)
}

#[cfg(test)]
mod tests;

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
