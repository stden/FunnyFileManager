//! Unit tests: every write to a vault — a save, a delete, an undo — leaves
//! the vault's permissions as they were, and a vault this process may not
//! write is refused rather than replaced.
//!
//! Unix-only: modes and `faccessat` are what is under test.

// The parent's `cfg(all(test, unix))` is not a plain `cfg(test)`, which is
// the only form Clippy recognises as test code.
#![cfg(test)]

use std::fs;
use std::os::unix::fs::PermissionsExt as _;

use tempfile::TempDir;

use super::*;

fn details(title: &str, password: &str) -> EntryDetails {
    EntryDetails {
        title: title.into(),
        username: "u".into(),
        url: String::new(),
        notes: String::new(),
        password: password.into(),
    }
}

/// A one-entry vault at `mode`, created through the production writer.
fn vault(dir: &TempDir, mode: u32) -> PathBuf {
    let path = dir.path().join("v.kdbx");
    create_database(&path, "pw", None, &[details("Existing", "old")]).expect("test setup");
    fs::set_permissions(&path, fs::Permissions::from_mode(mode)).expect("test setup");
    path
}

/// The permission bits, in octal as `ls` and `chmod` spell them.
fn mode(path: &Path) -> String {
    format!(
        "{:o}",
        fs::metadata(path).expect("metadata").permissions().mode() & 0o777
    )
}

/// Whether the suite runs with root's ability to ignore permissions: root may
/// write a read-only file, so the refusal cannot be observed under it.
fn running_as_root() -> bool {
    crate::platform::is_root()
}

/// The save renamed a fresh 0600 temp file over the vault, so a 0640 vault
/// (shared with a group on purpose) came back 0600 after every edit.
#[test]
fn a_save_keeps_the_vaults_mode() {
    let dir = TempDir::new().expect("temporary directory");
    let path = vault(&dir, 0o640);

    upsert_entry(&path, "pw", None, &details("Existing", "new")).expect("save");

    assert_eq!(mode(&path), "640");
}

/// Ctrl+Z restores the backup through the same rename, with the same loss.
#[test]
fn an_undo_keeps_the_vaults_mode() {
    let dir = TempDir::new().expect("temporary directory");
    let path = vault(&dir, 0o640);
    upsert_entry(&path, "pw", None, &details("Existing", "new")).expect("save");

    restore_backup(&path).expect("undo");

    assert_eq!(mode(&path), "640");
}

/// rename(2) checks only the directory, so a 0444 vault in a writable
/// directory was replaced in silence and came back 0600. A vault the user may
/// not write is now refused before anything — the backup included — is
/// touched. Root may write it, and then its mode is kept.
#[test]
fn a_read_only_vault_is_refused_and_keeps_its_mode() {
    let dir = TempDir::new().expect("temporary directory");
    let path = vault(&dir, 0o444);
    let before = fs::read(&path).expect("vault bytes");

    let result = upsert_entry(&path, "pw", None, &details("Existing", "new"));

    assert_eq!(mode(&path), "444", "the mode is never changed");
    if running_as_root() {
        result.expect("root may write a read-only vault");
        return;
    }
    let error = result.expect_err("a read-only vault must not be replaced");
    assert!(
        matches!(&error, KeepassError::Io(io) if io.kind() == std::io::ErrorKind::PermissionDenied),
        "{error}"
    );
    assert_eq!(fs::read(&path).expect("vault bytes"), before);
    assert!(
        !path.with_file_name("v.kdbx.bak").exists(),
        "nothing was written, not even the backup"
    );
}

/// The undo is refused the same way.
#[test]
fn an_undo_on_a_read_only_vault_is_refused() {
    if running_as_root() {
        return; // Root may write a read-only vault.
    }
    let dir = TempDir::new().expect("temporary directory");
    let path = vault(&dir, 0o640);
    upsert_entry(&path, "pw", None, &details("Existing", "new")).expect("save");
    fs::set_permissions(&path, fs::Permissions::from_mode(0o444)).expect("test setup");
    let before = fs::read(&path).expect("vault bytes");

    let error = restore_backup(&path).expect_err("a read-only vault must not be replaced");

    assert!(
        matches!(&error, KeepassError::Io(io) if io.kind() == std::io::ErrorKind::PermissionDenied),
        "{error}"
    );
    assert_eq!(fs::read(&path).expect("vault bytes"), before);
    assert_eq!(mode(&path), "444");
}
