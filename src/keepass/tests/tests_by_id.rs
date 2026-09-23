//! Purpose: entry identity: reading, updating and deleting by id when titles collide.

use super::*;
use tempfile::TempDir;

/// A root-group vault with two entries titled `GitHub`, and its listing.
/// Each entry's password is `pw-` followed by its user name.
fn twins() -> (TempDir, PathBuf, Vec<EntrySummary>) {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("v.kdbx");
    save_cheap(
        &path,
        "pw",
        None,
        &[
            entry("GitHub", "alice", "", "", "pw-alice"),
            entry("GitHub", "bob", "", "", "pw-bob"),
        ],
    );
    let list = list_entries(&path, "pw", None).expect("list");
    assert_eq!(list.len(), 2);
    (dir, path, list)
}

/// Titles are not unique. Two `GitHub` rows in one group are legal, and the
/// listing must hand back an identity that tells them apart — reading the
/// second by name would silently return the first.
#[test]
fn duplicate_titles_are_told_apart_by_id() {
    let (_dir, path, list) = twins();

    assert_ne!(list[0].id, list[1].id, "each row carries its own identity");
    for summary in &list {
        let details = read_entry_by_id(&path, "pw", None, summary.id).expect("read by id");
        assert_eq!(details.username, summary.username);
        assert_eq!(details.password, format!("pw-{}", summary.username));
    }
}

/// Writing by id changes that entry and leaves its namesake alone.
#[test]
fn update_by_id_touches_only_the_addressed_twin() {
    let (_dir, path, list) = twins();
    let bob = list
        .iter()
        .find(|s| s.username == "bob")
        .expect("bob listed");
    let alice = list
        .iter()
        .find(|s| s.username == "alice")
        .expect("alice listed");

    update_entry_by_id(
        &path,
        "pw",
        None,
        &[],
        bob.id,
        &entry("GitHub", "bob", "", "", "pw-bob-2"),
    )
    .expect("update by id");

    let password = |id| read_entry_by_id(&path, "pw", None, id).unwrap().password;
    assert_eq!(password(alice.id), "pw-alice");
    assert_eq!(password(bob.id), "pw-bob-2");
}

#[test]
fn delete_by_id_can_be_restored_from_the_write_backup() {
    let (_dir, path, list) = twins();
    let bob = list.iter().find(|entry| entry.username == "bob").unwrap();

    delete_entry_by_id(&path, "pw", None, bob.id).expect("delete by id");
    let remaining = list_entries(&path, "pw", None).expect("list after delete");
    assert_eq!(remaining.len(), 1);
    assert_eq!(remaining[0].username, "alice");

    restore_backup(&path).expect("restore backup");
    let restored = list_entries(&path, "pw", None).expect("list after undo");
    assert_eq!(restored.len(), 2);
    assert!(restored.iter().any(|entry| entry.username == "bob"));
}

#[test]
fn update_by_id_still_refuses_a_rename_onto_a_sibling() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("v.kdbx");
    save_cheap(
        &path,
        "pw",
        None,
        &[entry("A", "", "", "", "pa"), entry("B", "", "", "", "pb")],
    );
    let list = list_entries(&path, "pw", None).expect("list");
    let a = list.iter().find(|s| s.title == "A").expect("A listed");

    assert!(matches!(
        update_entry_by_id(&path, "pw", None, &[], a.id, &entry("b", "", "", "", "pa")),
        Err(KeepassError::AlreadyExists)
    ));
    assert!(
        read_entry_by_id(
            &path,
            "pw",
            None,
            list.iter().find(|s| s.title == "B").unwrap().id
        )
        .is_ok(),
        "the id outlives a refused write"
    );
}
