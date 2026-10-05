//! Purpose: The sessions file is written with the settings' guards — through
//! a symbolic link, never over another user's file, a hard-linked one or a
//! read-only one — and a file that did not load says why.

use super::{file_store, fs, TempDir, FILE_NAME};
// Links, hard links and owners are Unix's; only their tests need these.
#[cfg(unix)]
use {
    super::{session, SessionStore},
    crate::sessions::StoreError,
    std::path::Path,
};

/// A store at `path` holding one session, ready to save.
#[cfg(unix)]
fn changed_store(path: &Path) -> SessionStore {
    let mut store = SessionStore::load_from(path.to_owned());
    store.upsert(session("new.test"));
    store
}

/// A sessions file kept in a dotfiles repository: the save replaced the link
/// with a plain file, and the file it named never saw the change.
#[cfg(unix)]
#[test]
fn a_symlink_is_written_through_and_stays_a_link() {
    let dir = TempDir::new().expect("temp dir");
    let real = dir.path().join("dot").join(FILE_NAME);
    fs::create_dir_all(real.parent().expect("parent")).expect("mkdir");
    changed_store(&real).save().expect("the real file");
    let link = dir.path().join(FILE_NAME);
    std::os::unix::fs::symlink(&real, &link).expect("link");
    let mut store = SessionStore::load_from(link.clone());
    store.upsert(session("via-link.test"));
    store.save().expect("saved through the link");
    assert!(fs::symlink_metadata(&link).expect("stat").is_symlink());
    assert!(fs::read_to_string(&real)
        .expect("read")
        .contains("via-link.test"));
}

/// A rename detaches only the name it replaces: the other would keep the old
/// sessions.
#[cfg(unix)]
#[test]
fn a_hard_linked_file_is_refused_and_both_names_keep_the_text() {
    let dir = TempDir::new().expect("temp dir");
    let path = dir.path().join(FILE_NAME);
    fs::write(&path, r#"{"sessions":[]}"#).expect("write");
    let other = dir.path().join("other.json");
    fs::hard_link(&path, &other).expect("hard link");
    let error = changed_store(&path).save().expect_err("refused");
    assert!(matches!(error, StoreError::Write(_)), "{error:?}");
    assert_eq!(
        error.to_string(),
        "cannot write the sessions file: sessions.json has other hard links; \
         FFM will not replace it"
    );
    assert_eq!(
        fs::read_to_string(&other).expect("kept"),
        r#"{"sessions":[]}"#
    );
}

/// Under `sudo` with the user's home, a save handed the user's file to root,
/// readable by root only, and the user saw no sessions from then on. Only
/// root can give a file away, so only root can set this up.
#[cfg(unix)]
#[test]
fn another_users_file_is_refused_and_keeps_its_text_and_owner() {
    use std::os::unix::fs::MetadataExt as _;

    const ANOTHER_UID: u32 = 4_242_424;
    if !crate::platform::is_root() {
        return;
    }
    let dir = TempDir::new().expect("temp dir");
    let path = dir.path().join(FILE_NAME);
    fs::write(&path, r#"{"sessions":[]}"#).expect("write");
    std::os::unix::fs::chown(&path, Some(ANOTHER_UID), None).expect("chown");
    let error = changed_store(&path).save().expect_err("refused");
    assert_eq!(
        error.to_string(),
        "cannot write the sessions file: sessions.json belongs to another user; \
         FFM will not replace it"
    );
    assert_eq!(
        fs::read_to_string(&path).expect("kept"),
        r#"{"sessions":[]}"#
    );
    assert_eq!(fs::metadata(&path).expect("stat").uid(), ANOTHER_UID);
}

/// A file that did not load keeps its reason, which every refused save and
/// the status line give.
#[test]
fn a_file_that_did_not_load_says_why() {
    let dir = TempDir::new().expect("temp dir");
    fs::create_dir(dir.path().join(FILE_NAME)).expect("a folder in its place");
    let store = file_store(&dir);
    let reason = "sessions.json did not load (not a regular file), so it is not overwritten; \
                  fix or move it first";
    assert_eq!(
        store.damage().map(|error| error.to_string()).as_deref(),
        Some(reason)
    );
    assert_eq!(
        store.save().map_err(|error| error.to_string()),
        Err(reason.to_owned())
    );
    assert!(file_store(&TempDir::new().expect("temp dir"))
        .damage()
        .is_none());
}
