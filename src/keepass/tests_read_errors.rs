//! Preserve filesystem failures across the third-party database reader boundary.

use std::io::{self, Read};

use tempfile::TempDir;

use super::*;

struct FailedRead;

impl Read for FailedRead {
    fn read(&mut self, _buffer: &mut [u8]) -> io::Result<usize> {
        Err(io::Error::new(
            io::ErrorKind::PermissionDenied,
            "read denied",
        ))
    }
}

#[test]
fn kdbx_read_failure_keeps_its_io_category_after_the_upstream_reader() {
    let error = Database::open(&mut FailedRead, DatabaseKey::new().with_password("unused"))
        .expect_err("the source cannot be read");
    let KeepassError::Io(error) = KeepassError::from(error) else {
        panic!("a read failure must not become a decryption failure");
    };
    assert_eq!(error.kind(), io::ErrorKind::PermissionDenied);
    assert_eq!(error.to_string(), "read denied");
}

#[test]
fn kdbx_missing_database_keeps_not_found_distinct_from_unreadable() {
    let dir = TempDir::new().expect("temporary directory");
    let error = list_entries(&dir.path().join("missing.kdbx"), "unused", None)
        .expect_err("missing database");
    assert!(matches!(error, KeepassError::Io(error) if error.kind() == io::ErrorKind::NotFound));
}

#[test]
fn kdbx_directory_read_is_an_io_error_even_when_open_succeeds() {
    let dir = TempDir::new().expect("temporary directory");
    let error = list_entries(dir.path(), "unused", None).expect_err("directory is not a vault");
    assert!(matches!(error, KeepassError::Io(error) if error.kind() != io::ErrorKind::NotFound));
}

#[test]
fn kdbx_keyfile_directory_read_keeps_its_io_category() {
    let dir = TempDir::new().expect("temporary directory");
    let error = build_key("unused", Some(dir.path())).expect_err("directory is not a key file");
    assert!(matches!(error, KeepassError::Io(error) if error.kind() != io::ErrorKind::NotFound));
}
