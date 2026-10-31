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
