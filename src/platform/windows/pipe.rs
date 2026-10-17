//! Purpose: The local named pipe between FFM and the helper it started as an
//! administrator: FFM creates it, the helper connects, and each checks the
//! process at the other end before trusting it.

use std::ffi::OsStr;
use std::fs::File;
use std::io;
use std::os::windows::fs::OpenOptionsExt;
use std::os::windows::io::{AsRawHandle, FromRawHandle, OwnedHandle};
use std::time::{Duration, Instant};

use windows_sys::Win32::Foundation::{GetLastError, ERROR_PIPE_CONNECTED, INVALID_HANDLE_VALUE};
use windows_sys::Win32::Storage::FileSystem::{
    FILE_FLAG_FIRST_PIPE_INSTANCE, PIPE_ACCESS_DUPLEX, SECURITY_IDENTIFICATION,
};
use windows_sys::Win32::System::Pipes::{
    ConnectNamedPipe, CreateNamedPipeW, GetNamedPipeClientProcessId, GetNamedPipeServerProcessId,
    PIPE_READMODE_BYTE, PIPE_REJECT_REMOTE_CLIENTS, PIPE_TYPE_BYTE, PIPE_WAIT,
};

use super::nul_terminated;

/// The pipe's buffer each way: a request or a progress report fits many times.
const BUFFER: u32 = 64 * 1024;

/// `\\.\pipe\<name>`.
fn pipe_path(name: &str) -> String {
    format!(r"\\.\pipe\{name}")
}

/// Create the pipe `name`, owned by this process: the first and only
/// instance, so a program that made one of that name first makes this fail,
/// and none can be made beside it. Local clients only.
///
/// # Errors
///
/// The OS error, `name` taken included.
#[allow(unsafe_code, reason = "std cannot create a named pipe")]
pub fn create_pipe(name: &str) -> io::Result<File> {
    let path = nul_terminated(OsStr::new(&pipe_path(name)))?;
    // SAFETY: `path` is NUL-terminated UTF-16 that outlives the call; no
    // security attributes, so the default ones (this user, administrators,
    // SYSTEM) apply.
    let handle = unsafe {
        CreateNamedPipeW(
            path.as_ptr(),
            PIPE_ACCESS_DUPLEX | FILE_FLAG_FIRST_PIPE_INSTANCE,
            PIPE_TYPE_BYTE | PIPE_READMODE_BYTE | PIPE_WAIT | PIPE_REJECT_REMOTE_CLIENTS,
            1,
            BUFFER,
            BUFFER,
            0,
            std::ptr::null(),
        )
    };
    if handle == INVALID_HANDLE_VALUE {
        return Err(io::Error::last_os_error());
    }
    // SAFETY: a fresh handle nothing else owns.
    Ok(File::from(unsafe { OwnedHandle::from_raw_handle(handle) }))
}

/// Wait for a client to connect to `pipe`, which [`create_pipe`] made.
///
/// Blocks until one does: the caller runs this on a thread of its own and
/// unblocks it, when it gives up, by connecting itself ([`open_pipe`]).
///
/// # Errors
///
/// The OS error of a failed wait.
#[allow(unsafe_code, reason = "std cannot wait for a named pipe's client")]
pub fn accept(pipe: &File) -> io::Result<()> {
    // SAFETY: `pipe` is a pipe handle this process created; no overlapped
    // structure, so the call blocks and writes nothing of ours.
    if unsafe { ConnectNamedPipe(pipe.as_raw_handle(), std::ptr::null_mut()) } != 0 {
        return Ok(());
    }
    // SAFETY: reads this thread's last error; no memory.
    match unsafe { GetLastError() } {
        // Connected between the creation and the wait: still a connection.
        ERROR_PIPE_CONNECTED => Ok(()),
        _ => Err(io::Error::last_os_error()),
    }
}

/// Open the pipe `name` as its client, retrying while its server is not
/// there yet, until `patience` runs out.
///
/// # Errors
///
/// The last OS error once `patience` is spent.
pub fn open_pipe(name: &str, patience: Duration) -> io::Result<File> {
    let started = Instant::now();
    loop {
        match std::fs::OpenOptions::new()
            .read(true)
            .write(true)
            .security_qos_flags(SECURITY_IDENTIFICATION)
            .open(pipe_path(name))
        {
            Ok(file) => return Ok(file),
            Err(error) if started.elapsed() >= patience => return Err(error),
            Err(_) => std::thread::sleep(Duration::from_millis(50)),
        }
    }
}

/// The id of the process at the client end of `pipe`.
///
/// # Errors
///
/// The OS error.
#[allow(
    unsafe_code,
    reason = "std cannot ask who is at the other end of a pipe"
)]
pub fn client_process_id(pipe: &File) -> io::Result<u32> {
    let mut id = 0u32;
    // SAFETY: `pipe` is a live pipe handle and `id` a valid place for the id.
    if unsafe { GetNamedPipeClientProcessId(pipe.as_raw_handle(), &raw mut id) } == 0 {
        return Err(io::Error::last_os_error());
    }
    Ok(id)
}

/// The id of the process at the server end of `pipe`.
///
/// # Errors
///
/// The OS error.
#[allow(
    unsafe_code,
    reason = "std cannot ask who is at the other end of a pipe"
)]
pub fn server_process_id(pipe: &File) -> io::Result<u32> {
    let mut id = 0u32;
    // SAFETY: `pipe` is a live pipe handle and `id` a valid place for the id.
    if unsafe { GetNamedPipeServerProcessId(pipe.as_raw_handle(), &raw mut id) } == 0 {
        return Err(io::Error::last_os_error());
    }
    Ok(id)
}

#[cfg(test)]
mod tests;
