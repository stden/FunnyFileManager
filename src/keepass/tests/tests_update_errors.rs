//! Purpose: `update_entry` refusals: a missing entry or group, and a wrong
//! password.

use super::*;
use tempfile::TempDir;

#[test]
fn update_entry_missing_is_entry_not_found() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("v.kdbx");
    save_cheap(&path, "pw", None, &[entry("X", "", "", "", "p")]);

    assert!(matches!(
        update_entry(
            &path,
            "pw",
            None,
            &[],
            "nope",
            &entry("nope", "", "", "", "p")
        ),
        Err(KeepassError::EntryNotFound)
    ));
    assert!(matches!(
        update_entry(
            &path,
            "pw",
            None,
            &["Missing"],
            "X",
            &entry("X", "", "", "", "p")
        ),
        Err(KeepassError::EntryNotFound)
    ));
}

#[test]
fn update_entry_wrong_password_is_open_error() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("v.kdbx");
    save_cheap(&path, "right", None, &[entry("X", "", "", "", "p")]);

    assert!(matches!(
        update_entry(&path, "wrong", None, &[], "X", &entry("X", "", "", "", "p")),
        Err(KeepassError::Open(_))
    ));
}
