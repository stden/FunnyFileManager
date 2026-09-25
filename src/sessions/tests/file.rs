//! Purpose: The sessions file — round-trips through an owner-only file, and
//! is never written over when it did not load.

use super::{file_store, fs, imported, session, SessionStore, TempDir, FILE_NAME, MAX_FILE_BYTES};
use crate::sessions::file::LoadError;
use crate::sessions::{ReadError, StoreError};
use crate::test_support::EnvVarGuard;

/// The variable that names the config root on this platform.
const CONFIG_HOME: &str = if cfg!(windows) {
    "APPDATA"
} else {
    "XDG_CONFIG_HOME"
};

#[test]
fn save_and_load_round_trip_through_an_owner_only_file() {
    let dir = TempDir::new().expect("temp dir");
    let mut store = file_store(&dir);
    assert!(store.is_empty(), "no file yet");
    assert!(!store.is_damaged());
    store.upsert(session("a.test"));
    store.upsert(imported("b.test"));
    store.save().expect("save");

    let path = dir.path().join(FILE_NAME);
    assert!(path.is_file());
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mode = fs::metadata(&path).expect("metadata").permissions().mode() & 0o777;
        assert_eq!(mode, 0o600, "owner-only, as the history is");
    }
    let text = fs::read_to_string(&path).expect("read");
    assert!(text.starts_with("{\n  \"sessions\""), "{text}");
    assert_eq!(SessionStore::load_from(path), store);
}

#[test]
fn saving_creates_the_config_directory() {
    let dir = TempDir::new().expect("temp dir");
    let path = dir.path().join("ffm").join(FILE_NAME);
    let mut store = SessionStore::load_from(path.clone());
    store.upsert(session("a.test"));
    store.save().expect("save");
    assert!(path.is_file());
}

/// Every way a file can be unusable gives an empty store that refuses to
/// save, and the file is left exactly as it was.
#[test]
fn an_unusable_file_is_never_written_over() {
    let duplicate = format!(
        "{{\"sessions\":[{0},{0}]}}",
        serde_json::to_string(&session("a.test")).expect("json")
    );
    let bad_value = r#"{"sessions":[{"name":"x","protocol":"Sftp","host":"a b","port":22}]}"#;
    let too_large = "x".repeat(usize::try_from(MAX_FILE_BYTES + 1).expect("fits"));
    for contents in [
        "{ not json",
        bad_value,
        duplicate.as_str(),
        too_large.as_str(),
    ] {
        let dir = TempDir::new().expect("temp dir");
        let path = dir.path().join(FILE_NAME);
        fs::write(&path, contents).expect("write");
        let mut store = SessionStore::load_from(path.clone());
        assert!(store.is_empty());
        assert!(store.is_damaged());
        store.upsert(session("a.test"));
        assert!(matches!(store.save(), Err(StoreError::Damaged(_))));
        assert_eq!(fs::read_to_string(&path).expect("read"), contents);
    }
}

#[test]
fn a_directory_in_the_files_place_is_unusable_too() {
    let dir = TempDir::new().expect("temp dir");
    fs::create_dir(dir.path().join(FILE_NAME)).expect("mkdir");
    assert!(file_store(&dir).is_damaged());
}

#[test]
fn a_write_that_fails_is_reported_with_its_cause() {
    let dir = TempDir::new().expect("temp dir");
    let config = dir.path().join("ffm");
    let mut store = SessionStore::load_from(config.join(FILE_NAME));
    store.upsert(session("a.test"));
    // The config directory's place is taken by a file after the load.
    fs::write(&config, "").expect("write");
    let error = store.save().expect_err("the parent is a file");
    assert!(matches!(error, StoreError::Write(_)), "{error:?}");
    assert!(
        error
            .to_string()
            .starts_with("cannot write the sessions file: "),
        "{error}"
    );
    assert_eq!(
        StoreError::Damaged("two sessions share a name".to_owned()).to_string(),
        "sessions.json did not load (two sessions share a name), so it is not overwritten; \
         fix or move it first"
    );
}

#[test]
fn the_production_path_is_under_the_config_directory() {
    let home = TempDir::new().expect("temp dir");
    let _guard = EnvVarGuard::set(CONFIG_HOME, home.path());
    let path = SessionStore::path().expect("a path");
    assert_eq!(path, home.path().join("ffm").join(FILE_NAME));
    assert!(SessionStore::load().is_empty());
    fs::create_dir_all(home.path().join("ffm")).expect("mkdir");
    fs::write(&path, "[]").expect("write");
    assert!(
        SessionStore::load().is_damaged(),
        "a list is not the file's shape"
    );
}

/// Far more than a few sessions fit: the limit is a mebibyte, not a page.
#[test]
fn a_store_of_many_sessions_saves_and_loads_whole() {
    let dir = TempDir::new().expect("temp dir");
    let mut store = file_store(&dir);
    for index in 0..200 {
        store.upsert(session(&format!("host{index:03}.example.test")));
    }
    store.save().expect("save");
    let size = fs::metadata(dir.path().join(FILE_NAME))
        .expect("metadata")
        .len();
    assert!(size > 16 * 1024, "{size} bytes");
    let loaded = file_store(&dir);
    assert!(!loaded.is_damaged());
    assert_eq!(loaded.len(), 200);
}

/// Only a missing file is an empty store; a path that cannot even be
/// looked at is a file that did not load.
#[cfg(unix)]
#[test]
fn a_path_that_cannot_be_read_is_not_taken_for_a_missing_file() {
    let dir = TempDir::new().expect("temp dir");
    let blocker = dir.path().join("not-a-dir");
    fs::write(&blocker, "").expect("write");
    let path = blocker.join(FILE_NAME);
    assert!(matches!(
        crate::sessions::file::read_sessions(&path),
        Err(LoadError::Read(ReadError::Io(_)))
    ));
    assert!(SessionStore::load_from(path).is_damaged());
    assert!(
        matches!(crate::sessions::file::read_sessions(&dir.path().join(FILE_NAME)), Ok(sessions) if sessions.is_empty())
    );
}

#[test]
fn why_a_file_did_not_load_reads_as_one_line() {
    assert_eq!(
        LoadError::Read(ReadError::NotUtf8).to_string(),
        "not UTF-8 text"
    );
    assert_eq!(
        LoadError::DuplicateName.to_string(),
        "two sessions share a name"
    );
    let parse = serde_json::from_str::<serde_json::Value>("{").expect_err("broken");
    let text = parse.to_string();
    assert_eq!(LoadError::Parse(parse).to_string(), text);
}
