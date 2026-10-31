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
            let mut output = fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(path)?;
            if let Err(error) =
                std::io::copy(&mut staged, &mut output).and_then(|_| output.sync_all())
            {
                drop(output);
                drop(fs::remove_file(path)); // Only the file exclusively created above is cleaned up.
                return Err(error.into());
            }
        }
        FileLocation::Remote { link, path } => {
            link.lock()
                .upload(path, &mut staged, WriteMode::CreateNew)?;
        }
    }
    Ok(())
}
