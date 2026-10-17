//! Purpose: opening and reading vaults: password and key-file round trips, wrong passwords, missing entries and group paths.

use super::*;
use std::io::Write as _;
use tempfile::TempDir;

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

fn save_distinct_group_paths(path: &Path) {
    let mut db = cheap_db();
    {
        let mut root = db.root_mut();
        {
            let mut work = root.add_group();
            work.name = "Work".to_owned();
            let mut ci = work.add_group();
            ci.name = "CI".to_owned();
            write_entry_fields(&mut ci.add_entry(), &entry("Deploy", "", "", "", "nested"));
        }
        let mut literal = root.add_group();
        literal.name = "Work/CI".to_owned();
        write_entry_fields(
            &mut literal.add_entry(),
            &entry("Deploy", "", "", "", "literal"),
        );
    }
    let key = build_key("pw", None).expect("key");
    save_atomic(&db, path, key).expect("save");
}
