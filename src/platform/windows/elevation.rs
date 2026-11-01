//! Purpose: Whether FFM runs as an administrator, and starting a program as
//! one: Windows asks the user first (UAC), and the caller keeps the process
//! to check who it is and whether it still runs.

use std::ffi::OsStr;
use std::io;
use std::os::windows::io::{AsRawHandle, FromRawHandle, OwnedHandle};
use std::path::Path;

use windows_sys::Win32::Foundation::{CloseHandle, HANDLE, STILL_ACTIVE};
use windows_sys::Win32::Security::{
    GetTokenInformation, TokenElevation, TOKEN_ELEVATION, TOKEN_QUERY,
};
use windows_sys::Win32::System::Threading::{
    GetCurrentProcess, GetExitCodeProcess, GetProcessId, OpenProcessToken,
};
use windows_sys::Win32::UI::Shell::{
    ShellExecuteExW, SEE_MASK_FLAG_NO_UI, SEE_MASK_NOASYNC, SEE_MASK_NOCLOSEPROCESS,
    SHELLEXECUTEINFOW,
};
use windows_sys::Win32::UI::WindowsAndMessaging::SW_HIDE;

use super::nul_terminated;

/// Open a document using the registered Windows handler, without waiting for it.
#[allow(unsafe_code, reason = "std has no Windows file association API")]
pub fn open_associated(path: &Path) -> io::Result<()> {
    let verb = nul_terminated(OsStr::new("open"))?;
    let file = nul_terminated(path.as_os_str())?;
    let mut info = SHELLEXECUTEINFOW {
        cbSize: u32::try_from(size_of::<SHELLEXECUTEINFOW>()).unwrap_or(0),
        fMask: SEE_MASK_NOASYNC | SEE_MASK_FLAG_NO_UI,
        lpVerb: verb.as_ptr(),
        lpFile: file.as_ptr(),
        nShow: windows_sys::Win32::UI::WindowsAndMessaging::SW_SHOWNORMAL,
        ..SHELLEXECUTEINFOW::default()
    };
    // SAFETY: both strings are NUL-terminated and live through this call;
    // info is sized correctly, and no process handle is requested.
    if unsafe { ShellExecuteExW(&raw mut info) } == 0 {
        return Err(io::Error::last_os_error());
    }
    Ok(())
}

const ELEVATION_LAUNCH_MASK: u32 = SEE_MASK_NOCLOSEPROCESS | SEE_MASK_NOASYNC | SEE_MASK_FLAG_NO_UI;

/// Whether this process runs with an administrator's full rights. `false`
/// when that cannot be found out: then elevation is worth offering.
#[allow(unsafe_code, reason = "std cannot read the process token's elevation")]
#[must_use]
pub fn is_elevated() -> bool {
    let mut token: HANDLE = std::ptr::null_mut();
    // SAFETY: takes nothing and returns a pseudo-handle that needs no closing.
    let process = unsafe { GetCurrentProcess() };
    // SAFETY: `process` is this process' pseudo-handle, and `token` a valid
    // place for the handle the call opens.
    if unsafe { OpenProcessToken(process, TOKEN_QUERY, &raw mut token) } == 0 {
        return false;
    }
    let mut elevation = TOKEN_ELEVATION { TokenIsElevated: 0 };
    let mut written = 0u32;
    let size = u32::try_from(size_of::<TOKEN_ELEVATION>()).unwrap_or(0);
    // SAFETY: `token` was opened for query above; `elevation` is `size`
    // bytes the call may write, and `written` a valid place for the count.
    let read = unsafe {
        GetTokenInformation(
            token,
            TokenElevation,
            (&raw mut elevation).cast(),
            size,
            &raw mut written,
        )
    };
    // SAFETY: `token` is the handle opened above, closed once.
    unsafe { CloseHandle(token) };
    read != 0 && elevation.TokenIsElevated != 0
}

/// A program started as an administrator, which the caller did not wait for.
pub struct ElevatedProcess(OwnedHandle);

impl ElevatedProcess {
    /// Its process id, to check the other end of a pipe against.
    #[allow(unsafe_code, reason = "std has no process id for a raw process handle")]
    #[must_use]
    pub fn id(&self) -> u32 {
        // SAFETY: the handle is a live process handle owned by `self`.
        unsafe { GetProcessId(self.0.as_raw_handle()) }
    }

    /// Whether it has ended.
    #[allow(
        unsafe_code,
        reason = "std cannot read a raw process handle's exit code"
    )]
    #[must_use]
    pub fn has_exited(&self) -> bool {
        let mut code = 0u32;
        // SAFETY: the handle is a live process handle owned by `self`, and
        // `code` a valid place for its exit code.
        let read = unsafe { GetExitCodeProcess(self.0.as_raw_handle(), &raw mut code) };
        read == 0 || !matches!(i32::try_from(code), Ok(STILL_ACTIVE))
    }
}

/// Start `program` with `parameters` as an administrator, hidden, without
/// waiting for it.
///
/// Windows asks the user first. A user who says no is `ERROR_CANCELLED`
/// (1223) as the raw OS error.
///
/// # Errors
///
/// The OS error of a launch that did not happen, a refused prompt included.
#[allow(
    unsafe_code,
    reason = "only ShellExecuteEx's \"runas\" verb asks Windows to elevate a program"
)]
pub fn launch_as_administrator(program: &Path, parameters: &OsStr) -> io::Result<ElevatedProcess> {
    let verb = nul_terminated(OsStr::new("runas"))?;
    let file = nul_terminated(program.as_os_str())?;
    let arguments = nul_terminated(parameters)?;
    let mut info = shell_execute_info(&verb, &file, &arguments);
    // SAFETY: every string `info` points at is NUL-terminated UTF-16 that
    // outlives the call; `info` itself is a valid, sized structure the call
    // may write the process handle into.
    if unsafe { ShellExecuteExW(&raw mut info) } == 0 {
        return Err(io::Error::last_os_error());
    }
    if info.hProcess.is_null() {
        return Err(io::Error::other("Windows started no process"));
    }
    // SAFETY: SEE_MASK_NOCLOSEPROCESS left this process handle open for us
    // and nothing else owns it.
    Ok(ElevatedProcess(unsafe {
        OwnedHandle::from_raw_handle(info.hProcess)
    }))
}

fn shell_execute_info(verb: &[u16], file: &[u16], arguments: &[u16]) -> SHELLEXECUTEINFOW {
    SHELLEXECUTEINFOW {
        cbSize: u32::try_from(size_of::<SHELLEXECUTEINFOW>()).unwrap_or(0),
        fMask: ELEVATION_LAUNCH_MASK,
        lpVerb: verb.as_ptr(),
        lpFile: file.as_ptr(),
        lpParameters: arguments.as_ptr(),
        nShow: SW_HIDE,
        ..SHELLEXECUTEINFOW::default()
    }
}

#[cfg(test)]
mod tests;
