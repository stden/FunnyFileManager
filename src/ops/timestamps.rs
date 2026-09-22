//! Handles for timestamp changes, including Windows directories and read-only files.

use std::{fs, io, path::Path};

#[cfg(windows)]
pub(super) fn open(path: &Path) -> io::Result<fs::File> {
    use std::os::windows::fs::OpenOptionsExt;
    use windows_sys::Win32::Storage::FileSystem::{
        FILE_FLAG_BACKUP_SEMANTICS, FILE_WRITE_ATTRIBUTES,
    };

    fs::OpenOptions::new()
        .access_mode(FILE_WRITE_ATTRIBUTES)
        .custom_flags(FILE_FLAG_BACKUP_SEMANTICS)
        .open(path)
}

#[cfg(not(windows))]
pub(super) fn open(path: &Path) -> io::Result<fs::File> {
    fs::File::open(path).or_else(|_| fs::OpenOptions::new().write(true).open(path))
}
