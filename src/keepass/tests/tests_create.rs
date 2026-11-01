//! Purpose: `create_database`: a new vault is readable and an existing file is never clobbered.

use super::*;
use tempfile::TempDir;

#[test]
fn create_database_writes_a_readable_vault() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("new.kdbx");

    create_database(
        &path,
        "pw",
        None,
        &[entry("GitHub", "alice", "https://gh", "note", "s3cr3t!")],
    )
    .expect("create");

    assert!(path.exists());
    let list = list_entries(&path, "pw", None).expect("list");
    assert_eq!(list.len(), 1);
    assert_eq!(list[0].title, "GitHub");
    let details = read_entry(&path, "pw", None, &[], "GitHub").expect("read");
    assert_eq!(details.password, "s3cr3t!");
    assert_eq!(details.username, "alice");
}

#[test]
fn create_database_refuses_to_clobber() {
    // No KDF work happens on this path, so it stays fast despite the strong
    // default KDF that a real create_database would use.
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("v.kdbx");
    std::fs::write(&path, b"existing").unwrap();

    assert!(matches!(
        create_database(&path, "pw", None, &[entry("T", "", "", "", "p")]),
        Err(KeepassError::AlreadyExists)
    ));
    // The existing file must be left byte-for-byte untouched.
    assert_eq!(std::fs::read(&path).unwrap(), b"existing");
}
