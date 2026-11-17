//! Purpose: Vault fixtures: generated entries, a vault file of a given size and an unlocked
//! session.

use funny_file_manager::kdbx::{KdbxSession, Vault};
use funny_file_manager::keepass::{create_database, EntryDetails};
use std::path::{Path, PathBuf};
use std::time::Instant;
use tempfile::TempDir;

/// Vault sizes: a personal vault, a team vault, and one large enough that an
/// accidental quadratic in the filter would be unmistakable.
pub const SIZES: [usize; 3] = [100, 1_000, 5_000];

const PASSWORD: &str = "bench-master-password";

/// Entries spread over a few groups, with titles that share prefixes — the
/// shape a filter actually meets. Uniform titles would let a prefix test
/// short-circuit in a way real data does not.
fn entries(count: usize) -> Vec<EntryDetails> {
    (0..count)
        .zip(["api", "web", "db", "ssh"].into_iter().cycle())
        .map(|(index, kind)| EntryDetails {
            title: format!("service-{index:04}-{kind}"),
            username: format!("user{}@example.com", index % 50),
            url: format!("https://host-{:03}.example.com", index % 200),
            notes: String::new(),
            password: format!("secret-{index:06}"),
        })
        .collect()
}

/// A saved vault. Kept alive by its `TempDir`, so the caller must hold both.
#[expect(
    clippy::expect_used,
    reason = "criterion has no error channel: a failed fixture or measured call must stop the run"
)]
pub fn vault_file(count: usize) -> (TempDir, PathBuf) {
    let dir = TempDir::new().expect("vault benchmark dir");
    let path = dir.path().join("bench.kdbx");
    create_database(&path, PASSWORD, None, &entries(count)).expect("write benchmark vault");
    (dir, path)
}

pub fn unlocked(path: &Path, now: Instant) -> KdbxSession {
    let mut session = KdbxSession::new(path.to_path_buf());
    for ch in PASSWORD.chars() {
        session.type_char(ch, now);
    }
    assert!(session.unlock(now), "benchmark vault must unlock");
    session
}

#[expect(
    clippy::expect_used,
    reason = "criterion has no error channel: a failed fixture or measured call must stop the run"
)]
pub fn vault_of(session: &mut KdbxSession) -> &mut Vault {
    session.vault_mut().expect("vault is unlocked")
}

/// A multi-group vault (`groups` groups x `per_group` entries) with cheap KDF.
#[expect(
    clippy::expect_used,
    reason = "criterion has no error channel: vault fixture file must be written"
)]
pub fn grouped_vault_file(groups: usize, per_group: usize) -> (TempDir, PathBuf) {
    let dir = TempDir::new().expect("vault benchmark dir");
    let path = dir.path().join("grouped.kdbx");
    let mut db = keepass::Database::new();
    if let keepass::config::KdfConfig::Argon2 {
        iterations,
        memory,
        parallelism,
        ..
    } = &mut db.config.kdf_config
    {
        *iterations = 1;
        *parallelism = 1;
        *memory = 64 * 1024;
    }
    fill_groups(&mut db, groups, per_group);
    let mut file = std::fs::File::create(&path).expect("vault file");
    db.save(
        &mut file,
        keepass::DatabaseKey::new().with_password(PASSWORD),
    )
    .expect("save vault");
    (dir, path)
}

fn fill_groups(db: &mut keepass::Database, groups: usize, per_group: usize) {
    use keepass::db::fields;
    let mut root = db.root_mut();
    for g in 0..groups {
        let mut group = root.add_group();
        group.name = format!("team-{g:02}");
        for i in 0..per_group {
            let mut entry = group.add_entry();
            entry.set_unprotected(fields::TITLE, format!("svc-{g:02}-{i:02}"));
            entry.set_unprotected(fields::USERNAME, format!("user-{i:02}@corp.example"));
            entry.set_protected(fields::PASSWORD, "secret");
        }
    }
}
