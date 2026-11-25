//! Unit tests for dangling links and unsupported-version save errors in `keepass::atomic`.

use super::*;
use keepass::config::DatabaseVersion;
use tempfile::TempDir;

#[test]
fn dangling_links_are_refused_by_restore_backup_and_save_atomic() {
    let dir = TempDir::new().expect("temp dir");
    let link = dir.path().join("vault.kdbx");
    crate::test_support::make_dangling_link(&dir.path().join("missing"), &link)
        .expect("dangling link");

    assert!(matches!(restore_backup(&link), Err(KeepassError::Io(_))));
    let key = DatabaseKey::new().with_password("pw");
    assert!(matches!(
        save_atomic(&Database::new(), &link, key),
        Err(KeepassError::Io(_))
    ));
}

#[test]
fn save_atomic_surfaces_serialization_errors_for_unsupported_versions() {
    let dir = TempDir::new().expect("temp dir");
    let path = dir.path().join("v3.kdbx");
    let mut db = Database::new();
    db.config.version = DatabaseVersion::KDB3(0);
    let key = DatabaseKey::new().with_password("pw");
    assert!(matches!(
        save_atomic(&db, &path, key),
        Err(KeepassError::Save(_))
    ));
}
