//! Win32 directory and volume identity queries for native volume maintenance.

use super::nul_terminated;
use std::{
    ffi::OsString,
    io,
    os::windows::ffi::OsStringExt,
    path::{Path, PathBuf},
};
use windows_sys::Win32::{
    Globalization::{MultiByteToWideChar, CP_OEMCP},
    Storage::FileSystem::{GetVolumeNameForVolumeMountPointW, GetVolumePathNameW},
    System::SystemInformation::GetSystemDirectoryW,
};

const PATH_CAPACITY: u16 = 32_768;

pub fn volume_mount_point(path: &Path) -> io::Result<PathBuf> {
    volume_query(path, GetVolumePathNameW).map(PathBuf::from)
}

pub fn volume_id(mount: &Path) -> io::Result<OsString> {
    volume_query(mount, GetVolumeNameForVolumeMountPointW)
}

/// Both volume APIs take a terminated input name and return a terminated output name.
#[allow(
    unsafe_code,
    reason = "Win32 exposes mount points and volume GUIDs through UTF-16 buffers"
)]
fn volume_query(
    path: &Path,
    query: unsafe extern "system" fn(*const u16, *mut u16, u32) -> i32,
) -> io::Result<OsString> {
    let input = nul_terminated(path.as_os_str())?;
    let mut buffer = vec![0; usize::from(PATH_CAPACITY)];
    // SAFETY: input is NUL-terminated, and buffer holds PATH_CAPACITY writable UTF-16 units.
    if unsafe {
        query(
            input.as_ptr(),
            buffer.as_mut_ptr(),
            u32::from(PATH_CAPACITY),
        )
    } == 0
    {
        return Err(io::Error::last_os_error());
    }
    buffer.truncate(
        buffer
            .iter()
            .position(|&unit| unit == 0)
            .unwrap_or(buffer.len()),
    );
    Ok(OsString::from_wide(&buffer))
}

#[allow(
    unsafe_code,
    reason = "the Windows system directory is independent of PATH and environment overrides"
)]
pub fn system_directory() -> io::Result<PathBuf> {
    let mut buffer = vec![0; usize::from(PATH_CAPACITY)];
    // SAFETY: buffer holds the indicated number of writable UTF-16 units.
    let length = unsafe { GetSystemDirectoryW(buffer.as_mut_ptr(), u32::from(PATH_CAPACITY)) };
    if length == 0 {
        return Err(io::Error::last_os_error());
    }
    if length >= u32::from(PATH_CAPACITY) {
        return Err(io::Error::other(
            "Windows system directory exceeds the path limit",
        ));
    }
    buffer.truncate(usize::try_from(length).map_err(io::Error::other)?);
    let directory = PathBuf::from(OsString::from_wide(&buffer));
    #[cfg(target_pointer_width = "32")]
    {
        // Under WOW64, System32 is redirected. Sysnative names the native system utilities.
        let native = directory.with_file_name("Sysnative");
        if native.is_dir() {
            return Ok(native);
        }
    }
    Ok(directory)
}

/// Decode the machine's OEM code page when redirected output uses its console encoding.
#[allow(
    unsafe_code,
    reason = "the active OEM code page and its conversion are supplied by Windows"
)]
pub fn decode_oem(bytes: &[u8]) -> String {
    let Ok(count) = i32::try_from(bytes.len()) else {
        return String::from_utf8_lossy(bytes).into_owned();
    };
    let mut wide = vec![0; bytes.len()];
    // SAFETY: input has count readable bytes and wide has count writable code units.
    let written = unsafe {
        MultiByteToWideChar(CP_OEMCP, 0, bytes.as_ptr(), count, wide.as_mut_ptr(), count)
    };
    if written <= 0 {
        return String::from_utf8_lossy(bytes).into_owned();
    }
    wide.truncate(usize::try_from(written).unwrap_or_default());
    String::from_utf16_lossy(&wide)
}
