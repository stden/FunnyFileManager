//! Purpose: User YAML sessions share a document without overwriting unrelated settings.
use super::{fs, session, SessionStore, TempDir};

#[test]
fn yaml_sessions_preserve_other_sections_and_migrate_legacy_entries() {
    let dir = TempDir::new().expect("temp dir");
    let legacy = dir.path().join("sessions.json");
    let mut old = SessionStore::load_from(legacy.clone());
    old.upsert(session("old.example.test"));
    old.save().expect("legacy save");
    let path = dir.path().join("ffm.yml");
    let mut store = SessionStore::load_from(path.clone());
    assert_eq!(store.len(), 1);
    store.upsert(session("new.example.test"));
    store.save().expect("YAML save");
    assert!(legacy.exists());
    let text = fs::read_to_string(&path).expect("YAML");
    assert!(text.contains("sessions:"));
    fs::write(&path, format!("{text}\nkeymap: {{custom: true}}\n")).expect("other section");
    store.remove(&session("old.example.test").name);
    store.save().expect("preserve other section");
    let document: serde_norway::Value =
        serde_norway::from_str(&fs::read_to_string(&path).expect("read")).expect("parse");
    assert_eq!(document["keymap"]["custom"].as_bool(), Some(true));
    assert_eq!(SessionStore::load_from(path).len(), 1);
}

#[test]
fn stale_yaml_session_save_refuses_to_replace_newer_servers() {
    let dir = TempDir::new().expect("temp dir");
    let path = dir.path().join("ffm.yml");
    fs::write(&path, "sessions: []\n").expect("initial");
    let mut first = SessionStore::load_from(path.clone());
    let mut second = SessionStore::load_from(path.clone());
    first.upsert(session("first.example.test"));
    first.save().expect("first");
    second.upsert(session("second.example.test"));
    assert!(second.save().is_err());
    assert!(SessionStore::load_from(path)
        .find(&session("first.example.test").name)
        .is_some());
}

#[test]
fn non_regular_or_oversized_yaml_session_store_refuses_to_save() {
    let dir = TempDir::new().expect("temp dir");
    let dir_target = dir.path().join("ffm.yml");
    fs::create_dir(&dir_target).expect("mkdir");
    let mut damaged = SessionStore::load_from(dir_target);
    assert!(damaged.save().is_err());

    let huge = dir.path().join("huge.yml");
    fs::write(
        &huge,
        format!(
            "pad: '{}'\nsessions: []\n",
            "x".repeat(usize::try_from(super::MAX_FILE_BYTES - 16).unwrap())
        ),
    )
    .expect("seed near cap");
    let mut store = SessionStore::load_from(huge);
    store.upsert(session("overflow.example.test"));
    assert!(store.save().is_err());
}

#[test]
fn saving_more_yaml_sessions_than_max_file_bytes_returns_too_large() {
    let dir = TempDir::new().expect("temp dir");
    let path = dir.path().join("ffm.yml");
    let many: Vec<_> = (0..12_000)
        .map(|idx| session(&format!("host-{idx:05}.very-long-domain.example.test")))
        .collect();
    assert!(matches!(
        super::super::yaml::save(&path, &[], &many),
        Err(super::super::StoreError::TooLarge { .. })
    ));
}
