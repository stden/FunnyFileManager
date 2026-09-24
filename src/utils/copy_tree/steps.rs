//! Purpose: the filesystem steps of a directory copy, each with its forced-failure test seam.

use std::{fs, io, path::Path};

use crate::utils::{copy_file, FileMode};

#[cfg(any(test, tarpaulin))]
use crate::utils::{
    forced_copy_step, COPY_DIR_ALL_FAIL_COPY_FILE, COPY_DIR_ALL_FAIL_CREATE,
    COPY_DIR_ALL_FAIL_ENTRY, COPY_DIR_ALL_FAIL_FILE_TYPE,
};

pub(super) fn create_dir_all_checked(dst: &Path) -> io::Result<()> {
    #[cfg(any(test, tarpaulin))]
    {
        if forced_copy_step() == COPY_DIR_ALL_FAIL_CREATE {
            return Err(io::Error::other("forced create_dir_all failure"));
        }
    }
    fs::create_dir_all(dst)
}

pub(super) fn read_dir_entry(entry: io::Result<fs::DirEntry>) -> io::Result<fs::DirEntry> {
    #[cfg(any(test, tarpaulin))]
    {
        if forced_copy_step() == COPY_DIR_ALL_FAIL_ENTRY {
            return Err(io::Error::other("forced read_dir entry failure"));
        }
    }
    entry
}

pub(super) fn entry_file_type(entry: &fs::DirEntry) -> io::Result<fs::FileType> {
    #[cfg(any(test, tarpaulin))]
    {
        if forced_copy_step() == COPY_DIR_ALL_FAIL_FILE_TYPE {
            return Err(io::Error::other("forced file_type failure"));
        }
    }
    entry.file_type()
}

pub(super) fn copy_file_checked(src: &Path, dst: &Path, mode: FileMode) -> io::Result<()> {
    #[cfg(any(test, tarpaulin))]
    {
        if forced_copy_step() == COPY_DIR_ALL_FAIL_COPY_FILE {
            return Err(io::Error::other("forced copy failure"));
        }
    }
    // The path goes into the message: the batch reports the folder, and only
    // this says which entry inside it was refused or failed.
    copy_file(src, dst, mode)
        .map_err(|error| io::Error::new(error.kind(), format!("{}: {error}", src.display())))
}
