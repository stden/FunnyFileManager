//! Unit tests for entry attachment names in `keepass::export`.

use super::*;
use keepass::config::KdfConfig;
use keepass::db::{fields, Value};
use keepass::{Database, DatabaseKey};
use tempfile::TempDir;

#[test]
fn export_includes_entry_attachment_names() {
    let dir = TempDir::new().expect("temp dir");
    let path = dir.path().join("attached.kdbx");
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
        *memory = 64 * 1024;
    }
    {
        let mut root = db.root_mut();
        let mut entry = root.add_entry();
        entry.set_unprotected(fields::TITLE, "WithAttachment");
        let _ = entry.add_attachment("secret.txt", Value::protected(b"data".to_vec()));
    }
    let key = DatabaseKey::new().with_password("pw");
    let mut file = std::fs::File::create(&path).expect("create vault");
    db.save(&mut file, key).expect("save vault");

    let exported = export(&path, "pw", None).expect("export");
    assert_eq!(exported.root.entries[0].attachments, vec!["secret.txt"]);
}
