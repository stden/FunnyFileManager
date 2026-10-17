//! Purpose: The raw Win32 calls behind the Windows platform wrappers: the
//! mounted-drive mask, a file's attribute bits and the broadcast that tells
//! running programs the environment changed; registry string values live in
//! `registry`, running as an administrator in `elevation`, and the pipe to
//! that administrator in `pipe`.

use std::ffi::OsStr;
use std::io;
use std::os::windows::ffi::OsStrExt;
use std::path::Path;

use windows_sys::Win32::Storage::FileSystem::{GetLogicalDrives, SetFileAttributesW};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    SendMessageTimeoutW, HWND_BROADCAST, SMTO_ABORTIFHUNG, WM_SETTINGCHANGE,
};

mod elevation;
mod pipe;
mod registry;

pub use elevation::{is_elevated, launch_as_administrator, ElevatedProcess};
pub use pipe::{accept, client_process_id, create_pipe, open_pipe, server_process_id};
pub use registry::{read_registry_string, update_registry_string, RegistryHive};

/// The drive letters Windows has mounted, as `GetLogicalDrives`' bitmask:
/// bit 0 is `A:`, bit 25 is `Z:`. Zero means the call failed.
#[allow(
    unsafe_code,
    reason = "std has no call listing the mounted drive letters"
)]
#[must_use]
pub fn logical_drives() -> u32 {
    // SAFETY: `GetLogicalDrives` takes no arguments, reads no caller memory
    // and only returns a bitmask.
    unsafe { GetLogicalDrives() }
}

/// Replace `path`'s attribute bits (`FILE_ATTRIBUTE_HIDDEN`, `…_READONLY`, …).
///
/// # Errors
///
/// `InvalidInput` for a path with an interior NUL — Win32 would silently act
/// on the part before it — and otherwise the OS error of the failed call.
#[allow(
    unsafe_code,
    reason = "std sets only the read-only bit; hidden, system and archive need Win32"
)]
pub fn set_file_attributes(path: &Path, attributes: u32) -> io::Result<()> {
    let wide = nul_terminated(path.as_os_str())?;
    // SAFETY: `wide` is a NUL-terminated UTF-16 path that outlives the call,
    // and the function reads nothing else.
    if unsafe { SetFileAttributesW(wide.as_ptr(), attributes) } == 0 {
        return Err(io::Error::last_os_error());
    }
    Ok(())
}

/// `text` as the NUL-terminated UTF-16 a Win32 call reads.
///
/// # Errors
///
/// `InvalidInput` for text with an interior NUL: Win32 would silently act on
/// the part before it.
fn nul_terminated(text: &OsStr) -> io::Result<Vec<u16>> {
    let mut wide: Vec<u16> = text.encode_wide().collect();
    if wide.contains(&0) {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "a name contains a NUL character",
        ));
    }
    wide.push(0);
    Ok(wide)
}

/// How long one window may take to answer the broadcast.
const BROADCAST_TIMEOUT_MS: u32 = 1000;

/// Tell every top-level window that the environment changed, as `setx` and
/// the System Properties dialog do, so Explorer starts new programs with it.
/// A window that hangs is skipped; one that is slow gets a second.
///
/// # Errors
///
/// The OS error when the broadcast fails or times out.
#[allow(
    unsafe_code,
    reason = "std cannot send a window message; WM_SETTINGCHANGE is how Windows announces a new environment"
)]
pub fn broadcast_environment_change() -> io::Result<()> {
    let environment = nul_terminated(OsStr::new("Environment"))?;
    // The message carries the section's name as a pointer in its LPARAM.
    let section = isize::try_from(environment.as_ptr().expose_provenance())
        .map_err(|error| io::Error::new(io::ErrorKind::InvalidInput, error))?;
    let mut answer: usize = 0;
    // SAFETY: `section` points at `environment`, NUL-terminated UTF-16 that
    // outlives the call, which only reads it; `answer` is a valid place for
    // the result.
    let sent = unsafe {
        SendMessageTimeoutW(
            HWND_BROADCAST,
            WM_SETTINGCHANGE,
            0,
            section,
            SMTO_ABORTIFHUNG,
            BROADCAST_TIMEOUT_MS,
            &raw mut answer,
        )
    };
    if sent == 0 {
        return Err(io::Error::last_os_error());
    }
    Ok(())
}
