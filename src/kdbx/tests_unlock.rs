//! Unlock-path tests: the `Locked` -> `Unlocked` transition, and what must hold
//! on either side of it.

use std::path::PathBuf;
use std::time::Instant;

use tempfile::TempDir;

use super::tests_support::{
    focus, index_of, sorted_visible_titles, type_password, unlocked_sample, vault_file,
    write_keyfile, SAMPLE, VAULT_PASSWORD,
};
use super::{KdbxError, KdbxSession, IDLE_LOCK};

#[test]
fn new_session_starts_locked_with_an_empty_prompt() {
    let session = KdbxSession::new(PathBuf::from("/nonexistent/vault.kdbx"));

    assert!(session.state.is_locked());
    assert!(session.vault().is_none());
    let prompt = session.prompt().expect("a new session is locked");
    assert!(prompt.is_empty());
    assert_eq!(prompt.masked(), "");
    assert_eq!(prompt.typed_len(), 0);
    assert!(prompt.error().is_none());
    assert!(prompt.keyfile().is_none());
}

#[test]
fn typed_password_is_exposed_only_as_one_bullet_per_character() {
    let mut session = KdbxSession::new(PathBuf::from("/nonexistent/vault.kdbx"));
    let now = Instant::now();

    // Multi-byte characters: the mask counts characters, not bytes.
    type_password(&mut session, "pä§", now);
    assert_eq!(session.prompt().expect("locked").masked(), "•••");
    assert_eq!(session.prompt().expect("locked").typed_len(), 3);

    session.backspace(now);
    assert_eq!(session.prompt().expect("locked").masked(), "••");
    assert_eq!(session.prompt().expect("locked").typed_len(), 2);
}

#[test]
fn unlock_with_the_right_password_lists_every_entry() {
    let (_dir, session, _now) = unlocked_sample();

    assert!(session.prompt().is_none(), "prompt is gone once unlocked");
    let vault = session.vault().expect("unlocked");
    assert_eq!(
        sorted_visible_titles(&session),
        vec!["GitHub", "Gmail", "Jenkins", "Router", "bank"]
    );
    assert_eq!(vault.matches(), [0, 1, 2, 3, 4]);
    assert_eq!(vault.selected(), 0);
    assert_eq!(vault.filter(), "");
    assert!(vault.revealed().is_none());
}

#[test]
fn wrong_password_stays_locked_records_the_error_and_clears_what_was_typed() {
    let (_dir, path) = vault_file(SAMPLE, VAULT_PASSWORD, None);
    let now = Instant::now();
    let mut session = KdbxSession::new(path);
    type_password(&mut session, "definitely not it", now);

    assert!(!session.unlock(now));

    assert!(session.vault().is_none(), "must not unlock");
    let prompt = session.prompt().expect("still locked");
    assert_eq!(prompt.error(), Some("Wrong password or key file"));
    // A wrong secret is still a secret: it must not be left sitting in the
    // prompt waiting for the next attempt.
    assert!(prompt.is_empty());
    assert_eq!(prompt.typed_len(), 0);
    assert_eq!(prompt.masked(), "");
}

#[test]
fn unlock_error_never_echoes_the_typed_password() {
    let (_dir, path) = vault_file(SAMPLE, VAULT_PASSWORD, None);
    let now = Instant::now();
    let mut session = KdbxSession::new(path);
    let typed = "swordfish-marmalade";
    type_password(&mut session, typed, now);

    assert!(!session.unlock(now));

    let message = session.prompt().expect("locked").error().expect("an error");
    assert!(!message.contains(typed), "leaked the password: {message}");
    assert!(
        !message.contains("swordfish"),
        "leaked a fragment: {message}"
    );
}

#[test]
fn typing_again_clears_the_previous_failure() {
    let (_dir, path) = vault_file(SAMPLE, VAULT_PASSWORD, None);
    let now = Instant::now();
    let mut session = KdbxSession::new(path);
    type_password(&mut session, "nope", now);
    assert!(!session.unlock(now));
    assert!(session.prompt().expect("locked").error().is_some());

    session.type_char('a', now);
    assert!(session.prompt().expect("locked").error().is_none());
}

#[test]
fn missing_file_is_reported_as_such_not_as_a_wrong_password() {
    let dir = TempDir::new().expect("temp dir");
    let now = Instant::now();
    let mut session = KdbxSession::new(dir.path().join("absent.kdbx"));
    type_password(&mut session, VAULT_PASSWORD, now);

    assert!(!session.unlock(now));
    assert_eq!(
        session.prompt().expect("locked").error(),
        Some("File not found")
    );
}

#[test]
fn a_key_file_vault_needs_the_key_file_and_keeps_it_for_later_reads() {
    let key_dir = TempDir::new().expect("temp dir");
    let keyfile = write_keyfile(&key_dir);
    let (_dir, path) = vault_file(SAMPLE, VAULT_PASSWORD, Some(&keyfile));
    let now = Instant::now();

    // The password alone must not open it.
    let mut without = KdbxSession::new(path.clone());
    type_password(&mut without, VAULT_PASSWORD, now);
    assert!(!without.unlock(now));
    assert_eq!(
        without.prompt().expect("locked").error(),
        Some("Wrong password or key file")
    );

    // With the key file it opens — and the key file has to survive into the
    // vault, because every password read re-opens the database.
    let mut with = KdbxSession::new(path);
    with.prompt_mut()
        .expect("locked")
        .set_keyfile(Some(keyfile));
    type_password(&mut with, VAULT_PASSWORD, now);
    assert!(with.unlock(now));
    assert_eq!(
        with.vault().expect("unlocked").entries().len(),
        SAMPLE.len()
    );
    focus(&mut with, "GitHub", now);
    assert_eq!(with.copy_password(now).expect("re-read"), "gh-pw");
}

#[test]
fn nested_entries_keep_their_group_path_and_stay_readable() {
    let (_dir, mut session, now) = unlocked_sample();

    let nested = &session.vault().expect("unlocked").entries()[index_of(&session, "Jenkins")];
    assert_eq!(nested.group_path, ["Work".to_owned(), "CI".to_owned()]);

    focus(&mut session, "Jenkins", now);
    // Reading it proves `group_path` is threaded through to `read_entry`:
    // resolving against the root group instead would not find this entry.
    assert_eq!(session.copy_password(now).expect("read nested"), "jk-pw");
}

#[test]
fn a_locked_session_refuses_every_read() {
    let (_dir, path) = vault_file(SAMPLE, VAULT_PASSWORD, None);
    let now = Instant::now();
    let mut session = KdbxSession::new(path);

    assert!(matches!(session.copy_password(now), Err(KdbxError::Locked)));
    assert!(matches!(session.copy_username(), Err(KdbxError::Locked)));
    assert!(matches!(session.reveal(now), Err(KdbxError::Locked)));
    assert!(!session.tick(now + IDLE_LOCK * 2), "nothing to auto-lock");
}

#[test]
fn unlock_on_an_already_unlocked_session_is_a_no_op_success() {
    let (_dir, mut session, now) = unlocked_sample();
    assert!(session.unlock(now), "already unlocked must report success");
    assert!(!session.state.is_locked());
}

#[test]
fn prompt_accessors_are_none_once_unlocked() {
    let (_dir, mut session, _now) = unlocked_sample();
    assert!(session.prompt().is_none());
    assert!(session.prompt_mut().is_none());
}

#[test]
fn unlocking_an_unreadable_path_reports_a_read_error_not_a_password_error() {
    let dir = TempDir::new().expect("temp dir");
    // A path whose parent component is a plain file fails with ENOTDIR at
    // open(), before any key material is looked at — the one I/O failure that
    // still reproduces under root, where permission bits are ignored.
    let blocker = dir.path().join("blocker");
    std::fs::write(&blocker, b"not a directory").expect("write");
    let now = Instant::now();
    let mut session = KdbxSession::new(blocker.join("vault.kdbx"));
    type_password(&mut session, VAULT_PASSWORD, now);

    assert!(!session.unlock(now));

    let error = session.prompt().expect("locked").error().expect("reason");
    assert!(error.starts_with("Cannot read file:"), "{error}");
}

#[test]
fn kdbx_error_display_is_fixed_and_secret_free() {
    assert_eq!(KdbxError::Locked.to_string(), "Vault is locked");
    assert_eq!(KdbxError::NoSelection.to_string(), "No entry selected");
    let wrapped = KdbxError::Keepass(crate::keepass::KeepassError::EntryNotFound);
    assert_eq!(wrapped.to_string(), "Cannot read entry: entry not found");
}

#[test]
fn unlock_error_message_maps_every_failure_class() {
    use crate::keepass::KeepassError;

    // Defensive arm: these variants cannot arise from a read, but the mapping
    // must stay total and fixed-text if they ever do.
    assert_eq!(
        super::unlock_error_message(&KeepassError::AlreadyExists),
        "destination already exists"
    );
    assert_eq!(
        super::unlock_error_message(&KeepassError::Io(std::io::Error::other("boom"))),
        "Cannot read file: boom"
    );
}
