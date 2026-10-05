//! Purpose: Verify that deleting the same `KeePass` entry twice reports a missing entry.

#![cfg(feature = "keepass")]

use funny_file_manager::keepass::{
    create_database, delete_entry_by_id, list_entries, EntryDetails, KeepassError,
};
use tempfile::TempDir;

#[test]
fn probe_second_delete_by_id_is_entry_not_found() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("v.kdbx");
    let details = EntryDetails {
        title: "GitHub".into(),
        username: "alice".into(),
        url: String::new(),
        notes: String::new(),
        password: "pw-alice".into(),
    };
    create_database(&path, "pw", None, std::slice::from_ref(&details)).expect("create");
    let list = list_entries(&path, "pw", None).expect("list");
    assert_eq!(list.len(), 1);
    let id = list[0].id;

    delete_entry_by_id(&path, "pw", None, id).expect("first delete");
    assert!(list_entries(&path, "pw", None)
        .expect("list again")
        .is_empty());

    let second = delete_entry_by_id(&path, "pw", None, id);
    assert!(
        matches!(second, Err(KeepassError::EntryNotFound)),
        "got {second:?}"
    );
}
