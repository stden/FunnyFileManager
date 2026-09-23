//! Purpose: `upsert_entry`: insert, in-place update with edit tracking, and the owner-only pre-write backup.

use super::*;
use tempfile::TempDir;

#[test]
fn upsert_adds_then_updates_and_backs_up() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("v.kdbx");
    save_cheap(&path, "pw", None, &[entry("Existing", "u", "", "", "old")]);

    // Insert a new entry.
    upsert_entry(&path, "pw", None, &entry("New", "n", "", "", "np")).expect("upsert add");
    assert_eq!(list_entries(&path, "pw", None).unwrap().len(), 2);

    // The pre-write database must have been backed up to <db>.bak.
    assert!(
        path.with_file_name("v.kdbx.bak").exists(),
        "expected a <db>.bak backup before the in-place write"
    );

    // Updating an existing title must not create a duplicate.
    upsert_entry(&path, "pw", None, &entry("Existing", "u", "", "", "newpw"))
        .expect("upsert update");
    assert_eq!(list_entries(&path, "pw", None).unwrap().len(), 2);
    let details = read_entry(&path, "pw", None, &[], "Existing").unwrap();
    assert_eq!(details.password, "newpw");
}

/// Regression: the update path used to write fields through `EntryMut`, which
/// leaves `times.last_modification` untouched and records no history. A `KeePass`
/// sync then rejects the edit (`EntryModificationTimeNotUpdated`) and keeps the
/// peer's older password.
#[test]
fn upsert_update_advances_modification_time_and_records_history() {
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

    upsert_entry(&path, "pw", None, &entry("Existing", "u", "", "", "newpw")).expect("upsert");

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

    // The previous state must survive in the entry's history so the old
    // password can still be recovered.
    let reopened = open_db(&path, "pw", None).expect("reopen");
    let root = reopened.root();
    let stored = root.entry_by_name("Existing").expect("entry");
    let history = stored.history.as_ref().expect("history");
    assert_eq!(history.get_entries().len(), 1);
    assert_eq!(history.get_entries()[0].get_password(), Some("old"));
}

/// Regression: the backup used `fs::copy`, which propagates the source mode, so
/// a 0644 vault produced a world-readable `<db>.bak`.
#[cfg(unix)]
#[test]
fn backup_is_owner_only_even_when_the_vault_is_not() {
    use std::os::unix::fs::PermissionsExt as _;

    let dir = TempDir::new().unwrap();
    let path = dir.path().join("v.kdbx");
    save_cheap(&path, "pw", None, &[entry("Existing", "u", "", "", "old")]);
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o644)).unwrap();

    upsert_entry(&path, "pw", None, &entry("Existing", "u", "", "", "newpw")).expect("upsert");

    let backup = path.with_file_name("v.kdbx.bak");
    let mode = std::fs::metadata(&backup).unwrap().permissions().mode() & 0o777;
    assert_eq!(mode, 0o600, "backup must not inherit the vault's mode");
    // It is still a usable copy of the pre-write database.
    assert_eq!(
        read_entry(&backup, "pw", None, &[], "Existing")
            .expect("read backup")
            .password,
        "old"
    );
}
