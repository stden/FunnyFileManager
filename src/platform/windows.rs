//! Purpose: The raw Win32 calls behind the Windows platform wrappers: the
//! mounted-drive mask and a file's attribute bits.

use std::io;
use std::os::windows::ffi::OsStrExt;
use std::path::Path;

use windows_sys::Win32::Storage::FileSystem::{GetLogicalDrives, SetFileAttributesW};

/// The drive letters Windows has mounted, as `GetLogicalDrives`' bitmask:
/// bit 0 is `A:`, bit 25 is `Z:`. Zero means the call failed.
#[allow(unsafe_code)]
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
#[allow(unsafe_code)]
pub fn set_file_attributes(path: &Path, attributes: u32) -> io::Result<()> {
    let mut wide: Vec<u16> = path.as_os_str().encode_wide().collect();
    if wide.contains(&0) {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "path contains a NUL character",
        ));
    }
    wide.push(0);
    // SAFETY: `wide` is a NUL-terminated UTF-16 path that outlives the call,
    // and the function reads nothing else.
    if unsafe { SetFileAttributesW(wide.as_ptr(), attributes) } == 0 {
        return Err(io::Error::last_os_error());
    }
    Ok(())
}
