//! Shared fixtures for the `kdbx` unit tests: real `.kdbx` files on disk, built
//! with a deliberately cheap Argon2 KDF so the suite stays fast.
//!
//! The cheap-KDF trick mirrors `src/keepass/tests.rs::cheap_db`; that helper is
//! private to its own `#[cfg(test)]` module, so it is re-stated here rather than
//! shared. Test fixtures duplicating production shapes is expected, not a DRY
//! violation.

use std::fs::File;
use std::io::Write as _;
use std::path::{Path, PathBuf};
use std::time::Instant;

use keepass::config::KdfConfig;
use keepass::db::fields;
use keepass::{Database, DatabaseKey};
use tempfile::TempDir;

use super::KdbxSession;

/// Master password of every fixture vault.
///
/// Deliberately made of ordinary words: `unlock_error_never_echoes_the_typed_password`
/// asserts that none of them shows up in the failure message.
pub const VAULT_PASSWORD: &str = "correct horse battery staple";

/// One entry to write into a fixture database.
pub struct Fixture {
    pub group: &'static [&'static str],
    pub title: &'static str,
    pub username: &'static str,
    pub url: &'static str,
    pub password: &'static str,
}

/// The five-entry vault most tests run against.
///
/// Chosen so each filter case is unambiguous:
/// `git` hits one title, `carol` only a user name, `192.168` only a URL,
/// `example` exactly two entries, and `Jenkins` lives two groups deep so
/// `group_path` threading is actually exercised.
///
/// Note that `keepass::list_entries` does **not** return a stable order: entries
/// within a group come back permuted differently on every run. Tests therefore
/// look positions up with [`index_of`] or pin the cursor with [`focus`] instead
/// of hard-coding indices.
pub const SAMPLE: &[Fixture] = &[
    Fixture {
        group: &[],
        title: "GitHub",
        username: "alice",
        url: "https://github.com",
        password: "gh-pw",
    },
    Fixture {
        group: &[],
        title: "Gmail",
        username: "bob@example.com",
        url: "https://mail.google.com",
        password: "gm-pw",
    },
    Fixture {
        group: &[],
        title: "Router",
        username: "admin",
        url: "192.168.1.1",
        password: "rt-pw",
    },
    Fixture {
        group: &[],
        title: "bank",
        username: "carol",
        url: "https://bank.example",
        password: "bk-pw",
    },
    Fixture {
        group: &["Work", "CI"],
        title: "Jenkins",
        username: "ci-bot",
        url: "https://ci.internal",
        password: "jk-pw",
    },
];

/// A database whose Argon2 cost is cut to near-zero.
///
/// `KdfConfig` is `#[non_exhaustive]`, so the default is mutated in place rather
/// than rebuilt.
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
        // mem_cost, which must be >= 8 * parallelism.
        *memory = 64 * 1024;
    }
    db
}

fn database_key(password: &str, keyfile: Option<&Path>) -> DatabaseKey {
    let mut key = DatabaseKey::new().with_password(password);
    if let Some(path) = keyfile {
        let mut file = File::open(path).expect("open key file");
        key = key.with_keyfile(&mut file).expect("read key file");
    }
    key
}

/// Create every group along `path` that does not exist yet.
fn ensure_group(db: &mut Database, path: &[&str]) {
    for depth in 1..=path.len() {
        if db.root().group_by_path(&path[..depth]).is_some() {
            continue;
        }
        let mut root = db.root_mut();
        let mut parent = root
            .group_by_path_mut(&path[..depth - 1])
            .expect("parent group exists");
        let mut group = parent.add_group();
        group.name = path[depth - 1].to_owned();
    }
}

/// Write `entries` into a fresh `.kdbx` inside a new `TempDir`.
///
/// The `TempDir` owns the file, so callers must keep it alive.
pub fn vault_file(
    entries: &[Fixture],
    password: &str,
    keyfile: Option<&Path>,
) -> (TempDir, PathBuf) {
    let dir = TempDir::new().expect("temp dir");
    let path = dir.path().join("vault.kdbx");
    let mut db = cheap_db();

    for fixture in entries {
        ensure_group(&mut db, fixture.group);
        let mut root = db.root_mut();
        let mut group = root
            .group_by_path_mut(fixture.group)
            .expect("group was just created");
        let mut entry = group.add_entry();
        entry.set_unprotected(fields::TITLE, fixture.title);
        entry.set_unprotected(fields::USERNAME, fixture.username);
        entry.set_unprotected(fields::URL, fixture.url);
        entry.set_protected(fields::PASSWORD, fixture.password);
    }

    let mut file = File::create(&path).expect("create vault");
    db.save(&mut file, database_key(password, keyfile))
        .expect("save vault");
    file.sync_all().expect("flush vault");
    (dir, path)
}

/// A 32-byte key file next to the vault.
pub fn write_keyfile(dir: &TempDir) -> PathBuf {
    let path = dir.path().join("vault.key");
    File::create(&path)
        .expect("create key file")
        .write_all(&[7u8; 32])
        .expect("write key file");
    path
}

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

fn fixture(title: &str) -> &'static Fixture {
    SAMPLE
        .iter()
        .find(|entry| entry.title == title)
        .unwrap_or_else(|| panic!("no fixture titled {title}"))
}

/// The password [`SAMPLE`] gave `title`.
pub fn password_for(title: &str) -> &'static str {
    fixture(title).password
}

/// The user name [`SAMPLE`] gave `title`.
pub fn username_for(title: &str) -> &'static str {
    fixture(title).username
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
