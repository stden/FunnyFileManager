//! Unit tests for `keepass`.

use super::*;

use keepass::config::KdfConfig;
use std::io::Write as _;
use tempfile::TempDir;

/// A database whose Argon2 KDF cost is cut to near-zero so round-trip
/// save/open stays fast in tests. `KdfConfig` is `#[non_exhaustive]`, so we
/// mutate the default in place rather than constructing a new value.
fn cheap_db() -> Database {
    let mut db = Database::new();
    if let KdfConfig::Argon2 {
        iterations,
        memory,
        parallelism,
        ..
    } = &mut db.config.kdf_config
    {
        *iterations = 1;
        *parallelism = 1;
        // `memory` is in bytes; keepass passes `memory / 1024` as argon2's KiB
        // mem_cost, which must be >= 8 * parallelism. 64 KiB is tiny and fast.
        *memory = 64 * 1024;
    }
    db
}

fn entry(title: &str, user: &str, url: &str, notes: &str, pw: &str) -> EntryDetails {
    EntryDetails {
        title: title.into(),
        username: user.into(),
        url: url.into(),
        notes: notes.into(),
        password: pw.into(),
    }
}

/// Build a cheap-KDF database with `entries` and save it to `path` through the
/// real `save_atomic` path (so tests exercise the production writer cheaply).
fn save_cheap(path: &Path, password: &str, keyfile: Option<&Path>, entries: &[EntryDetails]) {
    let mut db = cheap_db();
    {
        let mut root = db.root_mut();
        for details in entries {
            let mut e = root.add_entry();
            write_entry_fields(&mut e, details);
        }
    }
    let key = build_key(password, keyfile).expect("build key");
    save_atomic(&db, path, key).expect("save");
}

#[test]
fn round_trip_password_only() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("v.kdbx");
    save_cheap(
        &path,
        "pw",
        None,
        &[entry("GitHub", "alice", "https://gh", "note", "s3cr3t!")],
    );

    let list = list_entries(&path, "pw", None).expect("list");
    assert_eq!(list.len(), 1);
    assert_eq!(list[0].title, "GitHub");
    assert_eq!(list[0].username, "alice");
    assert!(list[0].group_path.is_empty());

    let details = read_entry(&path, "pw", None, &[], "GitHub").expect("read");
    assert_eq!(details.password, "s3cr3t!");
    assert_eq!(details.url, "https://gh");
    assert_eq!(details.notes, "note");
}

#[test]
fn round_trip_password_plus_keyfile() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("v.kdbx");
    let keyfile = dir.path().join("key.bin");
    std::fs::File::create(&keyfile)
        .unwrap()
        .write_all(&[7u8; 32])
        .unwrap();

    save_cheap(
        &path,
        "pw",
        Some(&keyfile),
        &[entry("Mail", "bob", "", "", "hunter2")],
    );

    let details = read_entry(&path, "pw", Some(&keyfile), &[], "Mail").expect("read w/ keyfile");
    assert_eq!(details.password, "hunter2");

    // The key file is genuinely required: the password alone must not open it.
    assert!(read_entry(&path, "pw", None, &[], "Mail").is_err());
}

#[test]
fn wrong_password_errors() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("v.kdbx");
    save_cheap(&path, "right", None, &[entry("X", "", "", "", "p")]);

    assert!(matches!(
        list_entries(&path, "wrong", None),
        Err(KeepassError::Open(_))
    ));
}

#[test]
fn missing_entry_and_missing_group_are_entry_not_found() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("v.kdbx");
    save_cheap(&path, "pw", None, &[entry("X", "", "", "", "p")]);

    assert!(matches!(
        read_entry(&path, "pw", None, &[], "nope"),
        Err(KeepassError::EntryNotFound)
    ));
    assert!(matches!(
        read_entry(&path, "pw", None, &["Missing"], "X"),
        Err(KeepassError::EntryNotFound)
    ));
}

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

/// Regression: group paths used to be `/`-joined strings, so a group literally
/// named `Work/CI` and the nesting `Work` -> `CI` collapsed onto the same key
/// and `read_entry` returned the wrong entry's password.
#[test]
fn nested_and_slash_named_groups_stay_distinct() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("v.kdbx");

    save_distinct_group_paths(&path);

    let list = list_entries(&path, "pw", None).expect("list");
    assert_eq!(list.len(), 2);
    assert!(list.iter().any(|e| e.group_path == ["Work", "CI"]));
    assert!(list.iter().any(|e| e.group_path == ["Work/CI"]));

    // Each summary's path must lead back to its own entry.
    assert_eq!(
        read_entry(&path, "pw", None, &["Work", "CI"], "Deploy")
            .expect("nested")
            .password,
        "nested"
    );
    assert_eq!(
        read_entry(&path, "pw", None, &["Work/CI"], "Deploy")
            .expect("literal")
            .password,
        "literal"
    );
}

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

#[test]
fn export_to_json_and_yaml_includes_all_fields() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("v.kdbx");

    save_custom_field_vault(&path);

    // Structured export captures every field, including the custom one and
    // the plaintext password.
    let exported = export(&path, "pw", None).expect("export");
    assert_eq!(exported.database_name.as_deref(), Some("My Vault"));
    assert_eq!(exported.root.entries.len(), 1);
    let entry = &exported.root.entries[0];
    assert_eq!(
        entry.fields.get("Title").map(String::as_str),
        Some("GitHub")
    );
    assert_eq!(
        entry.fields.get("Password").map(String::as_str),
        Some("s3cr3t!")
    );
    assert_eq!(
        entry.fields.get("API-Token").map(String::as_str),
        Some("custom-value")
    );

    // JSON round-trips back to the identical structure.
    let json = export_json(&path, "pw", None).expect("json");
    assert!(json.contains("\"API-Token\"") && json.contains("s3cr3t!"));
    let from_json: KdbxExport = serde_json::from_str(&json).unwrap();
    assert_eq!(from_json, exported);

    // YAML round-trips back to the identical structure.
    let yaml = export_yaml(&path, "pw", None).expect("yaml");
    assert!(yaml.contains("API-Token"));
    let from_yaml: KdbxExport = serde_norway::from_str(&yaml).unwrap();
    assert_eq!(from_yaml, exported);
}

/// The export must mirror the real group tree, not just the root group.
#[test]
fn export_includes_nested_groups() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("v.kdbx");

    let mut db = cheap_db();
    {
        let mut root = db.root_mut();
        let mut work = root.add_group();
        work.name = "Work".to_string();
        write_entry_fields(&mut work.add_entry(), &entry("Deploy", "", "", "", "p"));
    }
    let key = build_key("pw", None).expect("key");
    save_atomic(&db, &path, key).expect("save");

    let exported = export(&path, "pw", None).expect("export");
    assert!(exported.root.entries.is_empty());
    assert_eq!(exported.root.groups.len(), 1);
    assert_eq!(exported.root.groups[0].name, "Work");
    assert_eq!(exported.root.groups[0].entries.len(), 1);
}

#[test]
fn keepass_error_display_is_fixed_text_per_variant() {
    use super::KeepassError;
    let io_error = KeepassError::Io(std::io::Error::other("disk gone"));
    let cases: &[(KeepassError, &str)] = &[
        (
            KeepassError::Open("bad header".into()),
            "failed to open kdbx: bad header",
        ),
        (
            KeepassError::Save("read-only".into()),
            "failed to save kdbx: read-only",
        ),
        (io_error, "kdbx I/O error: disk gone"),
        (KeepassError::EntryNotFound, "entry not found"),
        (KeepassError::AlreadyExists, "destination already exists"),
        (
            KeepassError::Serialize("bad utf8".into()),
            "failed to serialize export: bad utf8",
        ),
    ];
    for (error, expected) in cases {
        assert_eq!(error.to_string(), *expected);
    }
}

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

#[test]
fn export_times_reports_expiry_only_when_the_flag_is_set() {
    let mut times = Times::default();
    times.expires = Some(false);
    times.expiry = times.creation;
    assert_eq!(
        export_times(&times).expires,
        None,
        "an expiry date without the flag means 'does not expire'"
    );

    times.expires = Some(true);
    let exported = export_times(&times);
    assert_eq!(
        exported.expires.is_some(),
        times.expiry.is_some(),
        "with the flag set the expiry timestamp is passed through"
    );
}

fn save_backdated_entry(path: &Path) {
    let mut db = cheap_db();
    {
        let mut root = db.root_mut();
        let mut e = root.add_entry();
        write_entry_fields(&mut e, &entry("Existing", "u", "", "", "old"));
        e.times.last_modification = Some(Times::epoch());
    }
    let key = build_key("pw", None).expect("key");
    save_atomic(&db, path, key).expect("save");
}

fn save_distinct_group_paths(path: &Path) {
    let mut db = cheap_db();
    {
        let mut root = db.root_mut();
        {
            let mut work = root.add_group();
            work.name = "Work".to_string();
            let mut ci = work.add_group();
            ci.name = "CI".to_string();
            write_entry_fields(&mut ci.add_entry(), &entry("Deploy", "", "", "", "nested"));
        }
        let mut literal = root.add_group();
        literal.name = "Work/CI".to_string();
        write_entry_fields(
            &mut literal.add_entry(),
            &entry("Deploy", "", "", "", "literal"),
        );
    }
    let key = build_key("pw", None).expect("key");
    save_atomic(&db, path, key).expect("save");
}

fn save_custom_field_vault(path: &Path) {
    let mut db = cheap_db();
    db.meta.database_name = Some("My Vault".to_string());
    {
        let mut root = db.root_mut();
        let mut e = root.add_entry();
        e.set_unprotected(fields::TITLE, "GitHub");
        e.set_unprotected(fields::USERNAME, "alice");
        e.set_protected(fields::PASSWORD, "s3cr3t!");
        e.set_unprotected("API-Token", "custom-value"); // a non-standard field
    }
    let key = build_key("pw", None).expect("key");
    save_atomic(&db, path, key).expect("save");
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
