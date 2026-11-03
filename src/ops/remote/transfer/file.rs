//! Purpose: Stage one file on disk so transfers between hosts never buffer it in memory.

use crate::models::remote_work::FileLocation;
use crate::remote::{RemoteError, WriteMode};
use std::{
    fs,
    io::{Seek, SeekFrom},
};

pub(super) fn copy_file(
    source: &FileLocation,
    destination: &FileLocation,
) -> Result<(), RemoteError> {
    let mut staged = tempfile::tempfile()?;
    match source {
        FileLocation::Local(path) => {
            std::io::copy(&mut crate::utils::open_regular_file(path)?, &mut staged)?;
        }
        FileLocation::Remote { link, path } => {
            link.lock().download(path, &mut staged)?;
        }
    }
    staged.seek(SeekFrom::Start(0))?;
    match destination {
        FileLocation::Local(path) => {
            write_created_local(path, &mut staged, |src, dst| {
                std::io::copy(src, dst).and_then(|_| dst.sync_all())
            })?;
        }
        FileLocation::Remote { link, path } => {
            link.lock()
                .upload(path, &mut staged, WriteMode::CreateNew)?;
        }
    }
    Ok(())
}

pub(in crate::ops::remote) fn write_created_local(
    path: &std::path::Path,
    staged: &mut fs::File,
    flush_copy: impl FnOnce(&mut fs::File, &mut fs::File) -> std::io::Result<()>,
) -> Result<(), RemoteError> {
    let mut output = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)?;
    if let Err(error) = flush_copy(staged, &mut output) {
        drop(output);
        drop(fs::remove_file(path)); // Only the file exclusively created above is cleaned up.
        return Err(error.into());
    }
    Ok(())
}
