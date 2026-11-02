//! Purpose: Fixture sessions unlocked against [`SAMPLE`], and the queries tests
//! ask of them — independent of the vault's unstable listing order.

use std::time::Instant;

use tempfile::TempDir;

use super::{vault_file, write_keyfile, SAMPLE, VAULT_PASSWORD};
use crate::kdbx::KdbxSession;

/// Type `password` into a locked session, one character at a time.
pub fn type_password(session: &mut KdbxSession, password: &str, now: Instant) {
    for ch in password.chars() {
        session.type_char(ch, now);
    }
}

/// A session unlocked against [`SAMPLE`], the moment it was unlocked, and the
/// `TempDir` that must outlive it.
pub fn unlocked_sample() -> (TempDir, KdbxSession, Instant) {
    let (dir, path) = vault_file(SAMPLE, VAULT_PASSWORD, None);
    let now = Instant::now();
    let mut session = KdbxSession::new(path);
    type_password(&mut session, VAULT_PASSWORD, now);
    assert!(session.unlock(now), "fixture vault must unlock");
    (dir, session, now)
}

/// Titles of the entries currently passing the filter, in display order.
pub fn visible_titles(session: &KdbxSession) -> Vec<String> {
    session
        .vault()
        .expect("vault is unlocked")
        .visible()
        .map(|entry| entry.title.clone())
        .collect()
}

/// Title under the cursor, or `None` when nothing matches.
pub fn selected_title(session: &KdbxSession) -> Option<String> {
    session
        .vault()
        .expect("vault is unlocked")
        .selected_entry()
        .map(|entry| entry.title.clone())
}

/// Position of `title` in the vault's (run-dependent) listing order.
pub fn index_of(session: &KdbxSession, title: &str) -> usize {
    session
        .vault()
        .expect("vault is unlocked")
        .entries()
        .iter()
        .position(|entry| entry.title == title)
        .unwrap_or_else(|| panic!("no entry titled {title}"))
}

/// Visible titles sorted, for comparisons that must not depend on listing order.
pub fn sorted_visible_titles(session: &KdbxSession) -> Vec<String> {
    let mut titles = visible_titles(session);
    titles.sort();
    titles
}

/// Filter down to exactly `title` so the cursor lands on a known entry whatever
/// the listing order happens to be. Every [`SAMPLE`] title is a unique needle.
pub fn focus(session: &mut KdbxSession, title: &str, now: Instant) {
    session
        .vault_mut()
        .expect("vault is unlocked")
        .set_filter(title, now);
    assert_eq!(
        selected_title(session).as_deref(),
        Some(title),
        "{title} should be the only match"
    );
}

/// A session unlocked against a key-file-protected [`SAMPLE`] vault.
///
/// Returns the vault's `TempDir`, the key file's `TempDir`, the session, and
/// the moment it was unlocked; both directories must outlive the session.
pub fn unlocked_sample_with_keyfile() -> (TempDir, TempDir, KdbxSession, Instant) {
    let key_dir = TempDir::new().expect("temp dir");
    let keyfile = write_keyfile(&key_dir);
    let (dir, path) = vault_file(SAMPLE, VAULT_PASSWORD, Some(&keyfile));
    let now = Instant::now();
    let mut session = KdbxSession::new(path);
    session
        .prompt_mut()
        .expect("locked")
        .set_keyfile(Some(keyfile));
    type_password(&mut session, VAULT_PASSWORD, now);
    assert!(session.unlock(now), "fixture vault must unlock");
    (dir, key_dir, session, now)
}
