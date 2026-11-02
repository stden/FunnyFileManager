//! Windows ACL owners and hard-link counts captured for panel display.

use std::{
    collections::BTreeMap, fs, io, os::windows::io::AsRawHandle, path::Path, process::Command,
};
use windows_sys::Win32::Storage::FileSystem::{
    GetFileInformationByHandle, BY_HANDLE_FILE_INFORMATION,
};

pub(super) fn owners(directory: &Path) -> io::Result<BTreeMap<String, String>> {
    let script = "$result=@{}; Get-ChildItem -LiteralPath $env:FFM_OWNER_DIRECTORY -Force | ForEach-Object { $result[$_.Name]=(Get-Acl -LiteralPath $_.FullName -ErrorAction Stop).Owner }; ConvertTo-Json -InputObject $result -Compress";
    let mut command = Command::new("powershell.exe");
    command
        .args(["-NoProfile", "-NonInteractive", "-Command", script])
        .env("FFM_OWNER_DIRECTORY", directory);
    let output = crate::ops::menu::process::capture(&mut command)?;
    if !output.status.success() {
        return Err(io::Error::other(
            String::from_utf8_lossy(&output.stderr).into_owned(),
        ));
    }
    serde_json::from_slice(&output.stdout).map_err(io::Error::other)
}

#[allow(
    unsafe_code,
    reason = "GetFileInformationByHandle has no safe wrapper in std"
)]
fn file_info(path: &Path) -> Option<BY_HANDLE_FILE_INFORMATION> {
    let file = fs::File::open(path).ok()?;
    let mut information = BY_HANDLE_FILE_INFORMATION::default();
    // SAFETY: the API fills this owned output structure and only borrows our
    // live file handle for the call.
    let success = unsafe { GetFileInformationByHandle(file.as_raw_handle(), &raw mut information) };
    (success != 0).then_some(information)
}

pub(super) fn link_count(path: &Path) -> String {
    file_info(path)
        .map(|info| info.nNumberOfLinks.to_string())
        .unwrap_or_default()
}

#[cfg(test)]
pub fn has_multiple_links(path: &Path) -> bool {
    file_info(path).is_some_and(|info| info.nNumberOfLinks > 1)
}

pub fn are_same_hard_link(a: &Path, b: &Path) -> bool {
    let (Ok(meta_a), Ok(meta_b)) = (fs::symlink_metadata(a), fs::symlink_metadata(b)) else {
        return false;
    };
    if meta_a.is_symlink() || meta_b.is_symlink() {
        return false;
    }
    let (Some(info_a), Some(info_b)) = (file_info(a), file_info(b)) else {
        return false;
    };
    info_a.nNumberOfLinks > 1
        && info_a.dwVolumeSerialNumber == info_b.dwVolumeSerialNumber
        && info_a.nFileIndexHigh == info_b.nFileIndexHigh
        && info_a.nFileIndexLow == info_b.nFileIndexLow
}
