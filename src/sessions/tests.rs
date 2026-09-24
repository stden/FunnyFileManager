//! Unit tests: the store keeps one session per name, sorts for display,
//! adopts imports without overwriting the user's own entries, round-trips
//! through an owner-only file, and never writes over a file it could not
//! read.

use std::fs;
use std::path::PathBuf;

use tempfile::TempDir;

use super::import::{Imported, SkipReason, Skipped};
use super::{read_text, LoadError, ReadError, SessionStore, StoreError, FILE_NAME, MAX_FILE_BYTES};
use crate::models::{HostName, Origin, Port, Protocol, Session, SessionName, UserName};
use crate::test_support::EnvVarGuard;

/// The variable that names the config root on this platform.
const CONFIG_HOME: &str = if cfg!(windows) {
    "APPDATA"
} else {
    "XDG_CONFIG_HOME"
};

fn session(host: &str) -> Session {
    Session::new(
        Protocol::Sftp,
        HostName::new(host).expect("host"),
        Port::SSH,
        Some(UserName::new("u").expect("user")),
    )
}

fn imported(host: &str) -> Session {
    Session {
        origin: Origin::NetBoxFile(PathBuf::from("/srv/x/demo.netbox")),
        ..session(host)
    }
}

fn name(text: &str) -> SessionName {
    SessionName::new(text).expect("name")
}

fn names(store: &SessionStore) -> Vec<&str> {
    store.sorted().iter().map(|s| s.name.as_str()).collect()
}

fn file_store(dir: &TempDir) -> SessionStore {
    SessionStore::load_from(dir.path().join(FILE_NAME))
}

#[test]
fn upsert_keeps_one_session_per_name_and_sorted_orders_by_name() {
    let mut store = SessionStore::default();
    assert!(store.is_empty());
    store.upsert(session("z.test"));
    store.upsert(session("a.test"));
    assert!(!store.is_empty());
    let mut changed = session("z.test");
    changed.port = Port::new(2222).expect("port");
    changed.name = name("u@z.test");
    store.upsert(changed);
    assert_eq!(store.len(), 2);
    assert_eq!(names(&store), ["u@a.test", "u@z.test"]);
    assert_eq!(
        store.find(&name("u@z.test")).map(|s| s.port.get()),
        Some(2222)
    );
    assert!(store.remove(&name("u@a.test")).is_some());
    assert!(store.remove(&name("u@a.test")).is_none());
    assert_eq!(store.find(&name("u@a.test")), None);
    assert_eq!(store.len(), 1);
}

#[test]
fn adopt_keeps_the_users_entries_and_refreshes_imported_ones() {
    let mut store = SessionStore::default();
    store.upsert(session("manual.test"));
    store.upsert(imported("old.test"));
    let mut refreshed = imported("old.test");
    refreshed.port = Port::new(2200).expect("port");
    refreshed.name = name("u@old.test");

    let report = store.adopt(Imported {
        sessions: vec![refreshed, imported("manual.test"), imported("new.test")],
        skipped: vec![Skipped::new("junk", SkipReason::UnsupportedProtocol)],
        passwords_left_out: 2,
    });

    assert_eq!(report.imported, 2);
    assert_eq!(report.passwords_left_out, 2);
    assert_eq!(
        report.skipped,
        [
            Skipped::new("junk", SkipReason::UnsupportedProtocol),
            Skipped::new("u@manual.test", SkipReason::ManualEntryKept),
        ]
    );
    assert_eq!(report.to_string(), "2 imported, 2 skipped");
    assert_eq!(
        store.find(&name("u@old.test")).map(|s| s.port.get()),
        Some(2200)
    );
    assert!(store
        .find(&name("u@manual.test"))
        .is_some_and(|s| s.origin.is_manual()));
    assert_eq!(store.len(), 3);
}

#[test]
fn a_session_only_store_saves_nowhere_and_says_it_did() {
    let mut store = SessionStore::default();
    store.upsert(session("a.test"));
    assert!(store.save().is_ok());
    assert!(!store.is_damaged());
}

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
        assert!(matches!(store.save(), Err(StoreError::Damaged)));
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
        StoreError::Damaged.to_string(),
        "the sessions file did not load, so it is not overwritten; fix or move it first"
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

#[test]
fn read_text_refuses_what_it_must_not_open_or_hold() {
    let dir = TempDir::new().expect("temp dir");
    let file = dir.path().join("f");
    fs::write(&file, "héllo").expect("write");
    assert_eq!(read_text(&file, 16).expect("text"), "héllo");
    assert!(matches!(
        read_text(&file, 3),
        Err(ReadError::TooLarge { limit: 3 })
    ));
    assert!(matches!(
        read_text(dir.path(), 16),
        Err(ReadError::NotAFile)
    ));
    fs::write(&file, [0xff, 0xfe]).expect("write");
    assert!(matches!(read_text(&file, 16), Err(ReadError::NotUtf8)));
    let missing = read_text(&dir.path().join("missing"), 16).expect_err("missing");
    assert!(
        matches!(&missing, ReadError::Io(error) if error.kind() == std::io::ErrorKind::NotFound)
    );
    let texts = [
        (ReadError::NotAFile, "not a regular file"),
        (ReadError::TooLarge { limit: 7 }, "larger than 7 bytes"),
        (ReadError::NotUtf8, "not UTF-8 text"),
    ];
    for (error, text) in texts {
        assert_eq!(error.to_string(), text);
    }
    assert!(!missing.to_string().is_empty());
}

/// A FIFO is never opened: opening one with no writer blocks for ever.
#[cfg(unix)]
#[test]
fn read_text_does_not_block_on_a_fifo() {
    let dir = TempDir::new().expect("temp dir");
    let fifo = dir.path().join("pipe");
    crate::test_support::make_fifo(&fifo).expect("mkfifo");
    let outcome =
        crate::test_support::run_with_timeout(std::time::Duration::from_secs(5), move || {
            matches!(read_text(&fifo, 16), Err(ReadError::NotAFile))
        });
    assert_eq!(outcome, Some(true));
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
        super::read_sessions(&path),
        Err(LoadError::Read(ReadError::Io(_)))
    ));
    assert!(SessionStore::load_from(path).is_damaged());
    assert!(
        matches!(super::read_sessions(&dir.path().join(FILE_NAME)), Ok(sessions) if sessions.is_empty())
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
