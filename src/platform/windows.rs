//! Purpose: The raw Win32 calls behind the Windows platform wrappers: the
//! mounted-drive mask, a file's attribute bits, the environment broadcast,
//! registry string values, UAC elevation, named pipes, and live process/network
//! sampling and control.

use std::ffi::OsStr;
use std::io;
use std::os::windows::ffi::OsStrExt;
use std::path::Path;

use windows_sys::Win32::Storage::FileSystem::{GetLogicalDrives, SetFileAttributesW};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    SendMessageTimeoutW, HWND_BROADCAST, SMTO_ABORTIFHUNG, WM_SETTINGCHANGE,
};

mod elevation;
mod network;
mod network_addrs;
mod pipe;
mod process_build;
mod process_control;
mod processes;
mod registry;
mod system_info;

#[cfg(not(test))]
pub use elevation::open_associated;
pub use elevation::{is_elevated, launch_as_administrator, ElevatedProcess};
pub use network::{icmp_ping, read_network_adapters};
pub use network_addrs::read_net_endpoints;
pub use pipe::{accept, client_process_id, create_pipe, open_pipe, server_process_id};
pub use process_build::WinOwnerCache;
pub use process_control::{set_process_priority, terminate_process};
pub use processes::list_processes;
pub use registry::{read_registry_string, update_registry_string, RegistryHive};
#[cfg(test)]
pub use system_info::now_filetime_ticks;
pub use system_info::read_system_sample;

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
#[allow(
    unsafe_code,
    reason = "std cannot send a window message; WM_SETTINGCHANGE is how Windows announces a new environment"
)]
pub fn broadcast_environment_change() -> io::Result<()> {
    let environment = nul_terminated(OsStr::new("Environment"))?;
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
