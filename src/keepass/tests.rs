//! Unit tests for `keepass`.

use std::path::PathBuf;

use super::*;

use keepass::config::KdfConfig;

/// A database whose Argon2 KDF cost is cut to near-zero so round-trip
/// save/open stays fast in tests. `KdfConfig` is `#[non_exhaustive]`, so we
/// mutate the default in place rather than constructing a new value.
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
        // mem_cost, which must be >= 8 * parallelism. 64 KiB is tiny and fast.
        *memory = 64 * 1024;
    }
    db
}

fn entry(title: &str, user: &str, url: &str, notes: &str, pw: &str) -> EntryDetails {
    EntryDetails {
        title: title.into(),
        username: user.into(),
        url: url.into(),
        notes: notes.into(),
        password: pw.into(),
    }
}

/// Build a cheap-KDF database with `entries` and save it to `path` through the
/// real `save_atomic` path (so tests exercise the production writer cheaply).
fn save_cheap(path: &Path, password: &str, keyfile: Option<&Path>, entries: &[EntryDetails]) {
    let mut db = cheap_db();
    {
        let mut root = db.root_mut();
        for details in entries {
            let mut e = root.add_entry();
            write_entry_fields(&mut e, details);
        }
    }
    let key = build_key(password, keyfile).expect("build key");
    save_atomic(&db, path, key).expect("save");
}

fn save_backdated_entry(path: &Path) {
    let mut db = cheap_db();
    {
        let mut root = db.root_mut();
        let mut e = root.add_entry();
        write_entry_fields(&mut e, &entry("Existing", "u", "", "", "old"));
        e.times.last_modification = Some(Times::epoch());
    }
    let key = build_key("pw", None).expect("key");
    save_atomic(&db, path, key).expect("save");
}

#[cfg(test)]
mod tests_by_id;
#[cfg(test)]
mod tests_create;
#[cfg(test)]
mod tests_error_display;
#[cfg(test)]
mod tests_export;
#[cfg(test)]
mod tests_open;
#[cfg(test)]
mod tests_update_entry;
#[cfg(test)]
mod tests_update_errors;
#[cfg(test)]
mod tests_update_rename;
#[cfg(test)]
mod tests_upsert;

#[test]
fn entry_details_debug_redacts_password() {
    let details = entry("Title", "alice", "https://example.com", "note", "s3cr3t");
    let formatted = format!("{details:?}");
    assert!(formatted.contains("[REDACTED]"));
    assert!(!formatted.contains("s3cr3t"));
}
