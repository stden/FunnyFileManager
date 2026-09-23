//! Purpose: `update_entry`: in-place and nested edits, renames, edit tracking, custom fields, backups and errors.

use super::*;
use tempfile::TempDir;

/// Regression guard: `upsert_entry` addresses the root group only, so routing
/// an edit of a nested entry through it would add a root-group duplicate.
/// `update_entry` must edit the entry where it lives.
#[test]
fn update_entry_edits_a_nested_entry_in_place() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("v.kdbx");

    save_nested_deploy_entry(&path);

    update_entry(
        &path,
        "pw",
        None,
        &["Work", "CI"],
        "Deploy",
        &entry("Deploy", "d", "", "", "new"),
    )
    .expect("update");

    let list = list_entries(&path, "pw", None).expect("list");
    assert_eq!(list.len(), 1, "no root-group duplicate may appear");
    assert_eq!(list[0].group_path, ["Work", "CI"]);
    assert_eq!(
        read_entry(&path, "pw", None, &["Work", "CI"], "Deploy")
            .expect("read")
            .password,
        "new"
    );
}

/// Same sync-compatibility contract as `upsert_entry`: the edit must go through
/// `edit_tracking`, so `last_modification` advances and the prior state lands
/// in history — otherwise a `KeePass` sync rejects the edit and keeps the
/// peer's older value.
#[test]
fn update_entry_advances_mtime_and_records_history() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("v.kdbx");

    // Backdate the entry: Times::now() has one-second resolution, so a
    // freshly-created entry could otherwise share a timestamp with the update.
    save_backdated_entry(&path);

    let before = export(&path, "pw", None).expect("export before");
    let modified_before = before.root.entries[0]
        .times
        .modified
        .clone()
        .expect("modified timestamp");

    update_entry(
        &path,
        "pw",
        None,
        &[],
        "Existing",
        &entry("Existing", "u", "", "", "newpw"),
    )
    .expect("update");

    let after = export(&path, "pw", None).expect("export after");
    let modified_after = after.root.entries[0]
        .times
        .modified
        .clone()
        .expect("modified timestamp");
    assert!(
        modified_after > modified_before,
        "last_modification must advance on update: {modified_before} -> {modified_after}"
    );

    assert_previous_password_in_history(&path);
}

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

/// `write_entry_fields` sets only the five standard keys, so a custom field on
/// the entry must survive an edit untouched.
#[test]
fn update_entry_keeps_custom_fields() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("v.kdbx");

    let mut db = cheap_db();
    {
        let mut root = db.root_mut();
        let mut e = root.add_entry();
        write_entry_fields(&mut e, &entry("GitHub", "alice", "", "", "old"));
        e.set_unprotected("API-Token", "custom-value");
    }
    let key = build_key("pw", None).expect("key");
    save_atomic(&db, &path, key).expect("save");

    update_entry(
        &path,
        "pw",
        None,
        &[],
        "GitHub",
        &entry("GitHub", "alice", "", "", "new"),
    )
    .expect("update");

    let exported = export(&path, "pw", None).expect("export");
    let fields_map = &exported.root.entries[0].fields;
    assert_eq!(
        fields_map.get("API-Token").map(String::as_str),
        Some("custom-value"),
        "a custom field must survive an edit"
    );
    assert_eq!(fields_map.get("Password").map(String::as_str), Some("new"));
}

#[test]
fn update_entry_writes_a_backup() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("v.kdbx");
    save_cheap(&path, "pw", None, &[entry("X", "", "", "", "old")]);

    update_entry(&path, "pw", None, &[], "X", &entry("X", "", "", "", "new")).expect("update");

    let backup = path.with_file_name("v.kdbx.bak");
    assert!(
        backup.exists(),
        "expected a <db>.bak backup before the in-place write"
    );
    // The backup is a usable copy of the pre-write database.
    assert_eq!(
        read_entry(&backup, "pw", None, &[], "X")
            .expect("read backup")
            .password,
        "old"
    );
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

fn save_nested_deploy_entry(path: &Path) {
    let mut db = cheap_db();
    {
        let mut root = db.root_mut();
        let mut work = root.add_group();
        work.name = "Work".to_string();
        let mut ci = work.add_group();
        ci.name = "CI".to_string();
        write_entry_fields(&mut ci.add_entry(), &entry("Deploy", "d", "", "", "old"));
    }
    let key = build_key("pw", None).expect("key");
    save_atomic(&db, path, key).expect("save");
}

fn assert_previous_password_in_history(path: &Path) {
    let reopened = open_db(path, "pw", None).expect("reopen");
    let root = reopened.root();
    let stored = root.entry_by_name("Existing").expect("entry");
    let history = stored.history.as_ref().expect("history");
    assert_eq!(history.get_entries().len(), 1);
    assert_eq!(history.get_entries()[0].get_password(), Some("old"));
}
