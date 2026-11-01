//! Purpose: Starting the real `wsl.exe` from the Windows directory, never
//! through PATH, and the guard that stops it on timeout or setup failure.

use std::thread;
use std::time::{Duration, Instant};

use super::{capture, read_pipe, Captured, Exit, ManagedProcess, DRAIN_GRACE, POLL_INTERVAL};

struct ChildGuard(std::process::Child);

impl Drop for ChildGuard {
    fn drop(&mut self) {
        // Terminate on setup failure or timeout, without an unbounded wait if
        // Windows cannot stop the client. Linux descendants may still continue.
        if matches!(self.0.try_wait(), Ok(Some(_))) {
            return;
        }
        // Best effort: a failed kill still gets the bounded wait below.
        drop(self.0.kill());
        let deadline = Instant::now() + DRAIN_GRACE;
        while matches!(self.0.try_wait(), Ok(None)) && Instant::now() < deadline {
            thread::sleep(POLL_INTERVAL);
        }
    }
}

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

#[allow(
    unsafe_code,
    reason = "std cannot locate the Windows directory, and a PATH lookup could run another wsl.exe"
)]
fn windows_directory() -> Result<std::path::PathBuf, String> {
    use std::os::windows::ffi::OsStringExt;
    use windows_sys::Win32::System::SystemInformation::GetWindowsDirectoryW;

    let mut buffer = vec![0; 260];
    loop {
        let capacity = u32::try_from(buffer.len()).map_err(|error| error.to_string())?;
        // SAFETY: the buffer is writable for the exact capacity passed to Win32.
        let length = unsafe { GetWindowsDirectoryW(buffer.as_mut_ptr(), capacity) };
        if length == 0 {
            return Err(format!(
                "Cannot locate Windows: {}",
                std::io::Error::last_os_error()
            ));
        }
        let length = usize::try_from(length).map_err(|error| error.to_string())?;
        if length < buffer.len() {
            buffer.truncate(length);
            return Ok(std::ffi::OsString::from_wide(&buffer).into());
        }
        if length > 32_768 {
            return Err("Windows installation path is too long".to_owned());
        }
        buffer.resize(length + 1, 0);
    }
}

pub(super) fn command(windows_directory: &std::path::Path, args: &[&str]) -> std::process::Command {
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

pub(in crate::wsl) fn run(args: &[&str], timeout: Duration) -> Result<Captured, String> {
    let mut child = ChildGuard(
        command(&windows_directory()?, args)
            .spawn()
            .map_err(|error| format!("Cannot start WSL; ensure it is installed: {error}"))?,
    );
    let stdout = read_pipe(child.0.stdout.take().ok_or("WSL stdout pipe is missing")?)?;
    let stderr = read_pipe(child.0.stderr.take().ok_or("WSL stderr pipe is missing")?)?;
    capture(&mut child, &stdout, &stderr, timeout)
}
