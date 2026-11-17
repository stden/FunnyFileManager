use super::env::{env_name, export_passwords_as_env, quote_env_value, EnvNames};
use super::{export, ExportError, OutputFormat, MAX_DATABASE_BYTES};
use keepass::db::fields;
use keepass::Database;
use std::fs;
use tempfile::TempDir;

#[test]
fn vault_and_keyfile_type_and_size_limits_precede_decryption() {
    use super::{export, open_database, ExportError, OutputFormat, MAX_DATABASE_BYTES};
    let root = tempfile::tempdir().unwrap();
    assert!(matches!(
        export(root.path(), None, None, OutputFormat::Env),
        Err(ExportError::Decrypt)
    ));
    let absent = root.path().join("absent");
    assert!(matches!(
        open_database(&absent, None, Some(root.path())),
        Err(ExportError::InvalidKeyFile)
    ));
    let large = root.path().join("large.key");
    fs::File::create(&large)
        .unwrap()
        .set_len(MAX_DATABASE_BYTES + 1)
        .unwrap();
    assert!(matches!(
        open_database(&absent, None, Some(&large)),
        Err(ExportError::DatabaseTooLarge(_))
    ));
}

#[test]
fn a_small_compressed_vault_cannot_expand_beyond_the_text_output_limit() {
    use super::{export, ExportError, OutputFormat, MAX_OUTPUT_BYTES};
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("expanded.kdbx");
    let mut database = crate::tests_kdbx::cheap_database();
    database
        .root_mut()
        .add_entry()
        .set_unprotected(fields::PASSWORD, "a".repeat(MAX_OUTPUT_BYTES + 1));
    crate::tests_kdbx::save_database(
        &path,
        &database,
        keepass::DatabaseKey::new().with_password("synthetic"),
    );
    drop(database);
    let error = export(&path, Some("synthetic"), None, OutputFormat::Env).unwrap_err();
    assert!(matches!(error, ExportError::OutputTooLarge(size) if size > MAX_OUTPUT_BYTES));
}

#[test]
fn env_names_are_portable_and_never_empty() {
    assert_eq!(env_name("Work/API token"), "WORK_API_TOKEN");
    assert_eq!(env_name("trail___"), "TRAIL");
    assert_eq!(env_name("___"), "ENTRY");
    assert_eq!(env_name("Crème brûlée"), "CR_ME_BR_L_E");
}

#[test]
fn env_values_escape_shell_sensitive_characters() {
    assert_eq!(
        quote_env_value("a\\b\"c\n\r${TOKEN}\t"),
        "\"a\\\\b\\\"c\\n\\r\\${TOKEN}\\t\""
    );
}

#[test]
fn duplicate_env_names_receive_a_collision_safe_suffix() {
    let mut used = EnvNames::default();
    assert_eq!(used.unique("API"), "KDBX_API_PASSWORD");
    assert_eq!(used.unique("API"), "KDBX_API_2_PASSWORD");
    assert_eq!(used.unique("API"), "KDBX_API_3_PASSWORD");
    // A real entry named API_2 must not collide with the generated suffix.
    assert_eq!(used.unique("API_2"), "KDBX_API_2_2_PASSWORD");
}

#[test]
fn export_passwords_skips_entries_without_password_and_walks_groups() {
    let mut db = Database::new();
    {
        let mut root = db.root_mut();
        let mut no_pw = root.add_entry();
        no_pw.set_unprotected(fields::TITLE, "NoteOnly");
        let mut sub = root.add_group();
        sub.name = "Ops".to_owned();
        let mut with_pw = sub.add_entry();
        with_pw.set_protected(fields::PASSWORD, "s3cr\rt");
    }
    let rendered = export_passwords_as_env(&db);
    assert!(rendered.starts_with("KDBX_OPS_"), "{rendered}");
    assert!(rendered.ends_with("=\"s3cr\\rt\""), "{rendered}");
}

#[test]
fn export_rejects_directory_vault_and_invalid_or_oversized_keyfiles() {
    let dir = TempDir::new().expect("temp");
    assert!(matches!(
        export(dir.path(), None, None, OutputFormat::Env),
        Err(ExportError::Decrypt)
    ));
    let vault = dir.path().join("v.kdbx");
    fs::write(&vault, b"not-kdbx").expect("write vault");
    let subdir = dir.path().join("keydir");
    fs::create_dir(&subdir).expect("mkdir");
    assert!(matches!(
        export(&vault, None, Some(&subdir), OutputFormat::Json),
        Err(ExportError::InvalidKeyFile)
    ));
    let big_key = dir.path().join("big.key");
    fs::File::create(&big_key)
        .expect("create")
        .set_len(MAX_DATABASE_BYTES + 1)
        .expect("sparse");
    assert!(matches!(
        export(&vault, None, Some(&big_key), OutputFormat::Yaml),
        Err(ExportError::DatabaseTooLarge(_))
    ));
}
