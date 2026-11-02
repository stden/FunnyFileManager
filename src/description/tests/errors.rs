//! Purpose: Error and non-regular file branches when reading description sidecars.

use std::{fs, io};
use tempfile::TempDir;

use super::{default_names, read, text_of, try_read};
use crate::fileload::MAX_SMALL_FILE_BYTES;

#[test]
fn missing_dir_directory_sidecar_and_oversized_sidecar_are_handled() -> io::Result<()> {
    let temp = TempDir::new()?;
    let names = default_names();
    assert!(read(&temp.path().join("missing"), &names).is_empty());
    assert_eq!(text_of(&temp.path().join("missing"))?, "");

    let sidecar = temp.path().join("Descript.ion");
    fs::create_dir(&sidecar)?;
    assert_eq!(
        try_read(temp.path(), &names).unwrap_err().kind(),
        io::ErrorKind::InvalidInput
    );
    fs::remove_dir(&sidecar)?;

    fs::File::create(&sidecar)?.set_len(MAX_SMALL_FILE_BYTES + 1)?;
    assert_eq!(
        try_read(temp.path(), &names).unwrap_err().kind(),
        io::ErrorKind::InvalidData
    );
    Ok(())
}

#[cfg(windows)]
#[test]
fn exclusively_locked_sidecar_propagates_read_error() -> io::Result<()> {
    use std::os::windows::fs::OpenOptionsExt;
    let temp = TempDir::new()?;
    let sidecar = temp.path().join("Descript.ion");
    let _locked = fs::OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .share_mode(0)
        .open(&sidecar)?;
    assert!(try_read(temp.path(), &default_names()).is_err());
    Ok(())
}
