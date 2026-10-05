//! Purpose: `update_entry` renames: in place without a duplicate, and never
//! onto a sibling's title.

use super::*;
use tempfile::TempDir;

#[test]
fn update_entry_renames_without_duplicating() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("v.kdbx");
    save_cheap(
        &path,
        "pw",
        None,
        &[
            entry("Old", "u", "", "", "p"),
            entry("Other", "", "", "", "q"),
        ],
    );

    update_entry(
        &path,
        "pw",
        None,
        &[],
        "Old",
        &entry("New", "u", "", "", "p"),
    )
    .expect("rename");

    let list = list_entries(&path, "pw", None).expect("list");
    assert_eq!(list.len(), 2, "a rename must not change the entry count");
    assert!(list.iter().any(|e| e.title == "New"));
    assert!(!list.iter().any(|e| e.title == "Old"));
    assert_eq!(
        read_entry(&path, "pw", None, &[], "New")
            .expect("read")
            .password,
        "p"
    );
}

/// Renaming onto a title that already exists in the group is refused: the
/// by-name read path takes the first match, so a silent duplicate would
/// permanently shadow one of the two entries.
#[test]
fn rename_onto_existing_sibling_is_already_exists() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("v.kdbx");
    save_cheap(
        &path,
        "pw",
        None,
        &[entry("A", "", "", "", "pa"), entry("B", "", "", "", "pb")],
    );

    // "b" also pins the collision check as case-insensitive, matching the
    // case-insensitive lookup that would collapse the two entries on read.
    assert!(matches!(
        update_entry(&path, "pw", None, &[], "A", &entry("b", "", "", "", "pa")),
        Err(KeepassError::AlreadyExists)
    ));

    // The refused rename must leave the vault untouched.
    assert_eq!(
        read_entry(&path, "pw", None, &[], "A")
            .expect("read")
            .password,
        "pa"
    );
    assert_eq!(list_entries(&path, "pw", None).expect("list").len(), 2);
}
