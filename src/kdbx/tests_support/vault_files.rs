//! Purpose: Fixture `.kdbx` files on disk, built with a deliberately cheap
//! Argon2 KDF so the suite stays fast, and the key files that protect them.

use std::fs::File;
use std::io::Write as _;
use std::path::{Path, PathBuf};

use keepass::config::KdfConfig;
use keepass::db::fields;
use keepass::{Database, DatabaseKey};
use tempfile::TempDir;

use super::Fixture;

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
