//! Windows ACL owners and hard-link counts captured for panel display.

use std::{
    collections::BTreeMap, fs, io, os::windows::io::AsRawHandle, path::Path, process::Command,
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

#[allow(unsafe_code)]
pub(super) fn link_count(path: &Path) -> String {
    use windows_sys::Win32::Storage::FileSystem::{
        GetFileInformationByHandle, BY_HANDLE_FILE_INFORMATION,
    };
    let Ok(file) = fs::File::open(path) else {
        return String::new();
    };
    let mut information = BY_HANDLE_FILE_INFORMATION::default();
    // SAFETY: the API fills this owned output structure and only borrows our
    // live file handle for the call.
    let success = unsafe { GetFileInformationByHandle(file.as_raw_handle(), &raw mut information) };
    if success == 0 {
        String::new()
    } else {
        information.nNumberOfLinks.to_string()
    }
}
