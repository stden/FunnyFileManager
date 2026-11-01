//! Purpose: Keep saved session files inside the loader's byte limit without losing prior data.

use super::{file_store, fs, session, SessionStore, TempDir, FILE_NAME, MAX_FILE_BYTES};
use crate::models::{Auth, KeyFile};
use crate::sessions::{StoreError, StoredFile};

fn store_with_bytes(dir: &TempDir, bytes: u64) -> SessionStore {
    let mut store = file_store(dir);
    store.sessions.clear();
    for index in 0..260 {
        let mut entry = session(&format!("host{index:03}.example.test"));
        entry.auth = Auth::KeyFile(KeyFile::new("k").expect("key file"));
        store.upsert(entry);
    }
    let initial = serde_json::to_string_pretty(&StoredFile {
        sessions: store.sessions.clone(),
    })
    .expect("JSON");
    let mut remaining = usize::try_from(bytes).expect("fits") - initial.len();
    for entry in &mut store.sessions {
        let added = remaining.min(4095);
        entry.auth = Auth::KeyFile(KeyFile::new(&"k".repeat(added + 1)).expect("key file"));
        remaining -= added;
    }
    assert_eq!(remaining, 0, "requested size fits valid session fields");
    store
}

#[test]
fn files_below_or_at_the_load_limit_save_and_reload_whole() {
    for bytes in [MAX_FILE_BYTES - 1, MAX_FILE_BYTES] {
        let dir = TempDir::new().expect("temp dir");
        let mut store = store_with_bytes(&dir, bytes);
        store.save().expect("within the load limit");
        let path = dir.path().join(FILE_NAME);
        assert_eq!(fs::metadata(path).expect("metadata").len(), bytes);
        assert_eq!(file_store(&dir), store);
    }
}

#[test]
fn an_oversized_save_preserves_the_previous_file() {
    let dir = TempDir::new().expect("temp dir");
    let mut previous = file_store(&dir);
    previous.upsert(session("kept.example.test"));
    previous.save().expect("save previous");
    let path = dir.path().join(FILE_NAME);
    let before = fs::read(&path).expect("read previous");
    let mut oversized = store_with_bytes(&dir, MAX_FILE_BYTES + 1);

    let error = oversized
        .save()
        .expect_err("must reject an unreadable file");
    assert!(matches!(
        error,
        StoreError::TooLarge {
            limit: MAX_FILE_BYTES
        }
    ));
    assert_eq!(
        error.to_string(),
        "the sessions file would exceed 1048576 bytes"
    );

    assert_eq!(fs::read(path).expect("read after rejected save"), before);
    assert_eq!(file_store(&dir), previous);
}

#[test]
fn an_oversized_first_save_does_not_create_a_file() {
    let dir = TempDir::new().expect("temp dir");
    let mut store = store_with_bytes(&dir, MAX_FILE_BYTES + 1);

    assert!(matches!(
        store.save(),
        Err(StoreError::TooLarge {
            limit: MAX_FILE_BYTES
        })
    ));

    assert!(!dir.path().join(FILE_NAME).exists());
}
