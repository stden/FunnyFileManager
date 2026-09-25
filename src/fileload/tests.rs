//! Unit tests: the bounded, regular-file-only read shared by F3 and F4. Device
//! nodes and FIFOs are in `tests/tests_special_files.rs`, a file that grows
//! while it is read in `tests/tests_growing.rs`, configuration and data files
//! and the open that never waits in `tests/tests_small_files.rs`.

use super::{read_for_text_ui, read_regular, Loaded};

use crate::utils::MAX_FILE_SIZE;

use std::fs::{self, File};
use std::io;

use tempfile::TempDir;

mod tests_growing;
mod tests_small_files;
// FIFOs and device nodes exist only on Unix.
#[cfg(unix)]
mod tests_special_files;

#[test]
fn reads_a_regular_file_whole() -> io::Result<()> {
    let temp = TempDir::new()?;
    let path = temp.path().join("notes.txt");
    fs::write(&path, b"hello")?;

    let Loaded::Bytes(bytes) = read_for_text_ui(&path)? else {
        panic!("a regular file must load");
    };
    assert_eq!(bytes, b"hello");
    Ok(())
}

#[test]
fn refuses_a_directory() -> io::Result<()> {
    let temp = TempDir::new()?;
    assert!(matches!(read_for_text_ui(temp.path())?, Loaded::NotRegular));
    Ok(())
}

#[test]
fn reports_a_missing_file_as_an_error() {
    let temp = TempDir::new().expect("temp dir");
    let Err(error) = read_for_text_ui(&temp.path().join("ghost.txt")) else {
        panic!("a missing file must surface as io::Error, not a Loaded case");
    };
    assert_eq!(error.kind(), io::ErrorKind::NotFound);
}

#[test]
fn refuses_a_file_over_the_limit() -> io::Result<()> {
    let temp = TempDir::new()?;
    let path = temp.path().join("huge.txt");
    // Sparse: `set_len` records the size in the inode without writing a byte.
    File::create(&path)?.set_len(MAX_FILE_SIZE.bytes() + 1)?;

    let Loaded::TooLarge(size) = read_for_text_ui(&path)? else {
        panic!("an over-limit file must be refused");
    };
    assert_eq!(size, MAX_FILE_SIZE.bytes() + 1);
    Ok(())
}

/// The guard is `>`, not `>=`: a file of exactly the limit still loads, and the
/// `take(limit + 1)` bound must not cut its last byte off.
#[test]
fn loads_a_file_exactly_at_the_limit() -> io::Result<()> {
    let temp = TempDir::new()?;
    let path = temp.path().join("limit.txt");
    File::create(&path)?.set_len(MAX_FILE_SIZE.bytes())?;

    let Loaded::Bytes(bytes) = read_for_text_ui(&path)? else {
        panic!("a file at exactly the limit must load");
    };
    assert_eq!(
        u64::try_from(bytes.len()).expect("fits"),
        MAX_FILE_SIZE.bytes()
    );
    Ok(())
}

/// A symlink to a regular file is viewable — the *target*'s type is what
/// decides, so the guard must follow the link rather than refuse it.
#[cfg(unix)]
#[test]
fn follows_a_symlink_to_a_regular_file() -> io::Result<()> {
    let temp = TempDir::new()?;
    let target = temp.path().join("target.txt");
    fs::write(&target, b"through the link")?;
    let link = temp.path().join("link.txt");
    std::os::unix::fs::symlink(&target, &link)?;

    let Loaded::Bytes(bytes) = read_for_text_ui(&link)? else {
        panic!("a link to a regular file must load");
    };
    assert_eq!(bytes, b"through the link");
    Ok(())
}

/// The limit is the caller's: the same guards, a smaller budget.
#[test]
fn read_regular_holds_to_the_limit_it_is_given() {
    let dir = TempDir::new().expect("temp dir");
    let file = dir.path().join("f");
    fs::write(&file, "four").expect("write");
    assert!(matches!(read_regular(&file, 4), Ok(Loaded::Bytes(bytes)) if bytes == b"four"));
    assert!(matches!(read_regular(&file, 3), Ok(Loaded::TooLarge(4))));
    assert!(matches!(
        read_regular(dir.path(), 4),
        Ok(Loaded::NotRegular)
    ));
}
