//! Purpose: `update_entry`: in-place and nested edits, edit tracking, custom
//! fields and backups. Renames are in `tests_update_rename.rs`, refusals in
//! `tests_update_errors.rs`.

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

fn save_nested_deploy_entry(path: &Path) {
    let mut db = cheap_db();
    {
        let mut root = db.root_mut();
        let mut work = root.add_group();
        work.name = "Work".to_owned();
        let mut ci = work.add_group();
        ci.name = "CI".to_owned();
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
