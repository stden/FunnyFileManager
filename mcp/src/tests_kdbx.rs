//! In-process contract tests for the secret-bearing `kbdx` MCP tool.

use super::*;
use keepass::config::KdfConfig;
use keepass::db::fields;
use keepass::{Database, DatabaseKey};
use std::fs::File;
use tempfile::TempDir;

const MASTER_PASSWORD: &str = "synthetic-master-password";

fn server_at(dir: &TempDir) -> (FfmServer, PathBuf) {
    let root = dir.path().canonicalize().expect("canonicalize temp dir");
    (FfmServer::new(root.clone()), root)
}

fn cheap_database() -> Database {
    let mut database = Database::new();
    if let KdfConfig::Argon2 {
        iterations,
        memory,
        parallelism,
        ..
    } = &mut database.config.kdf_config
    {
        *iterations = 1;
        *parallelism = 1;
        *memory = 64 * 1024;
    }
    database
}

fn fixture_database() -> Database {
    let mut database = cheap_database();
    database.meta.database_name = Some("Synthetic Vault".to_owned());
    database.meta.database_description = Some("MCP test data".to_owned());
    {
        let mut root = database.root_mut();
        root.name = "Root".to_owned();

        let mut github = root.add_entry();
        github.set_unprotected(fields::TITLE, "GitHub");
        github.set_unprotected(fields::USERNAME, "alice");
        github.set_protected(fields::PASSWORD, "root-secret");
        github.set_unprotected("API-Token", "custom-secret");

        let mut work = root.add_group();
        work.name = "Work".to_owned();

        let mut first = work.add_entry();
        first.set_unprotected(fields::TITLE, "API token");
        first.set_protected(fields::PASSWORD, "a\\b\"c\n${TOKEN}\t");

        let mut second = work.add_entry();
        second.set_unprotected(fields::TITLE, "API-token");
        second.set_protected(fields::PASSWORD, "second-secret");
    }
    database
}

fn save_database(path: &Path, database: &Database, key: DatabaseKey) {
    let mut file = File::create(path).expect("create encrypted fixture");
    database
        .save(&mut file, key)
        .expect("save encrypted fixture");
}

fn save_password_fixture(path: &Path) {
    save_database(
        path,
        &fixture_database(),
        DatabaseKey::new().with_password(MASTER_PASSWORD),
    );
}

fn tool_text(result: &CallToolResult) -> &str {
    &result
        .content
        .first()
        .and_then(|content| content.as_text())
        .expect("a successful tool result has text content")
        .text
}

fn params(path: &Path, format: KdbxFormat) -> KdbxParams {
    KdbxParams {
        path: path.display().to_string(),
        password: Some(MASTER_PASSWORD.to_owned()),
        keyfile: None,
        format,
    }
}

fn entry_named<'a>(document: &'a serde_json::Value, title: &str) -> &'a serde_json::Value {
    document["entries"]
        .as_object()
        .expect("serialized database has an entry map")
        .values()
        .find(|entry| entry["fields"][fields::TITLE] == title)
        .expect("entry title is present")
}

#[test]
fn json_is_the_default_and_contains_all_decrypted_fields() {
    let dir = TempDir::new().expect("temp dir");
    let (server, root) = server_at(&dir);
    let vault = root.join("vault.KDBX");
    save_password_fixture(&vault);

    let decoded: KdbxParams = serde_json::from_value(json!({
        "path": vault,
        "password": MASTER_PASSWORD,
    }))
    .expect("deserialize tool arguments");
    assert!(matches!(decoded.format, KdbxFormat::Json));

    let result = server.kbdx(Parameters(decoded)).expect("JSON export");
    let document: serde_json::Value =
        serde_json::from_str(tool_text(&result)).expect("valid JSON response");

    assert_eq!(document["meta"]["database_name"], "Synthetic Vault");
    let github = entry_named(&document, "GitHub");
    assert_eq!(github["fields"][fields::PASSWORD], "root-secret");
    assert_eq!(github["fields"]["API-Token"], "custom-secret");
    assert!(
        document["groups"]
            .as_object()
            .expect("serialized database has a group map")
            .values()
            .any(|group| group["name"] == "Work"),
        "nested groups must be preserved"
    );
}

#[test]
fn yaml_contains_the_same_complete_model_as_json() {
    let dir = TempDir::new().expect("temp dir");
    let (server, root) = server_at(&dir);
    let vault = root.join("vault.kbdx");
    save_password_fixture(&vault);

    let json_result = server
        .kbdx(Parameters(params(&vault, KdbxFormat::Json)))
        .expect("JSON export");
    let yaml_result = server
        .kbdx(Parameters(params(&vault, KdbxFormat::Yaml)))
        .expect("YAML export");
    let from_json: serde_json::Value =
        serde_json::from_str(tool_text(&json_result)).expect("valid JSON");
    let from_yaml: serde_json::Value =
        serde_norway::from_str(tool_text(&yaml_result)).expect("valid YAML");

    assert_eq!(from_yaml, from_json);
    assert_eq!(
        entry_named(&from_yaml, "GitHub")["fields"][fields::PASSWORD],
        "root-secret"
    );
}

#[test]
fn env_contains_passwords_with_escaped_values_and_unique_names() {
    let dir = TempDir::new().expect("temp dir");
    let (server, root) = server_at(&dir);
    let vault = root.join("vault.kdbx");
    save_password_fixture(&vault);

    let result = server
        .kbdx(Parameters(params(&vault, KdbxFormat::Env)))
        .expect("dotenv export");
    let output = tool_text(&result);

    assert!(output.contains("KDBX_GITHUB_PASSWORD=\"root-secret\""));
    assert!(output.contains("KDBX_WORK_API_TOKEN_PASSWORD=\"a\\\\b\\\"c\\n\\${TOKEN}\\t\""));
    assert!(output.contains("KDBX_WORK_API_TOKEN_2_PASSWORD=\"second-secret\""));
    assert!(!output.contains("custom-secret"), "dotenv is password-only");
}

#[test]
fn password_plus_root_confined_keyfile_is_supported() {
    let dir = TempDir::new().expect("temp dir");
    let (server, root) = server_at(&dir);
    let vault = root.join("keyed.kdbx");
    let keyfile = root.join("vault.key");
    File::create(&keyfile)
        .expect("create key file")
        .write_all(&[7_u8; 32])
        .expect("write key file");
    let mut key_file_for_save = File::open(&keyfile).expect("open key file");
    let key = DatabaseKey::new()
        .with_password(MASTER_PASSWORD)
        .with_keyfile(&mut key_file_for_save)
        .expect("build composite key");
    save_database(&vault, &fixture_database(), key);

    let result = server
        .kbdx(Parameters(KdbxParams {
            path: vault.display().to_string(),
            password: Some(MASTER_PASSWORD.to_owned()),
            keyfile: Some(keyfile.display().to_string()),
            format: KdbxFormat::Json,
        }))
        .expect("composite credentials decrypt the database");

    assert_eq!(
        entry_named(
            &serde_json::from_str(tool_text(&result)).expect("valid JSON"),
            "GitHub"
        )["fields"][fields::PASSWORD],
        "root-secret"
    );
}

#[test]
fn database_and_keyfile_must_both_stay_inside_the_server_root() {
    let root_dir = TempDir::new().expect("root temp dir");
    let outside_dir = TempDir::new().expect("outside temp dir");
    let (server, root) = server_at(&root_dir);
    let inside_vault = root.join("inside.kdbx");
    let outside_vault = outside_dir.path().join("outside.kdbx");
    let outside_key = outside_dir.path().join("outside.key");
    save_password_fixture(&inside_vault);
    save_password_fixture(&outside_vault);
    File::create(&outside_key).expect("outside key fixture");

    let vault_error = server
        .kbdx(Parameters(params(&outside_vault, KdbxFormat::Json)))
        .expect_err("outside database must be refused");
    assert!(vault_error.message.contains("outside the allowed root"));

    let key_error = server
        .kbdx(Parameters(KdbxParams {
            path: inside_vault.display().to_string(),
            password: Some(MASTER_PASSWORD.to_owned()),
            keyfile: Some(outside_key.display().to_string()),
            format: KdbxFormat::Json,
        }))
        .expect_err("outside key file must be refused");
    assert!(key_error.message.contains("outside the allowed root"));
}

#[test]
fn invalid_inputs_do_not_leak_credentials() {
    let dir = TempDir::new().expect("temp dir");
    let (server, root) = server_at(&dir);
    let vault = root.join("vault.kdbx");
    save_password_fixture(&vault);
    let wrong_password = "this-must-not-appear-in-the-error";

    let error = server
        .kbdx(Parameters(KdbxParams {
            path: vault.display().to_string(),
            password: Some(wrong_password.to_owned()),
            keyfile: None,
            format: KdbxFormat::Json,
        }))
        .expect_err("wrong password must fail");

    assert!(error.message.contains("could not decrypt"));
    assert!(!error.message.contains(wrong_password));
}

#[test]
fn a_credential_and_a_vault_file_extension_are_required() {
    let dir = TempDir::new().expect("temp dir");
    let (server, root) = server_at(&dir);
    let text = root.join("not-a-vault.txt");
    File::create(&text).expect("text fixture");

    let credential_error = server
        .kbdx(Parameters(KdbxParams {
            path: text.display().to_string(),
            password: None,
            keyfile: None,
            format: KdbxFormat::Json,
        }))
        .expect_err("credential-less request must fail first");
    assert!(credential_error.message.contains("provide a password"));

    let extension_error = server
        .kbdx(Parameters(KdbxParams {
            path: text.display().to_string(),
            password: Some(MASTER_PASSWORD.to_owned()),
            keyfile: None,
            format: KdbxFormat::Json,
        }))
        .expect_err("non-vault extension must fail");
    assert!(extension_error.message.contains(".kdbx or .kbdx"));
}

#[test]
fn oversized_database_is_rejected_before_decryption() {
    let dir = TempDir::new().expect("temp dir");
    let (server, root) = server_at(&dir);
    let vault = root.join("large.kdbx");
    File::create(&vault)
        .expect("create sparse fixture")
        .set_len(kdbx::MAX_DATABASE_BYTES + 1)
        .expect("size sparse fixture");

    let error = server
        .kbdx(Parameters(params(&vault, KdbxFormat::Json)))
        .expect_err("oversized database must be refused");

    assert!(error.message.contains("maximum"));
    assert!(!error.message.contains(MASTER_PASSWORD));
}

#[test]
fn router_schema_advertises_formats_default_and_read_only_behavior() {
    let tool = FfmServer::tool_router()
        .list_all()
        .into_iter()
        .find(|tool| tool.name == "kbdx")
        .expect("kbdx is registered");
    let schema = serde_json::Value::Object((*tool.input_schema).clone());

    assert_eq!(schema["properties"]["format"]["default"], "json");
    let formats: Vec<&str> = schema["$defs"]["KdbxFormat"]["oneOf"]
        .as_array()
        .expect("format alternatives")
        .iter()
        .map(|variant| variant["const"].as_str().expect("string format name"))
        .collect();
    assert_eq!(
        formats,
        vec!["json", "yaml", "env"],
        "all accepted formats must be discoverable"
    );
    assert_eq!(schema["required"], json!(["path"]));

    let annotations = tool.annotations.expect("read-only annotations");
    assert_eq!(annotations.read_only_hint, Some(true));
    assert_eq!(annotations.destructive_hint, Some(false));
    assert_eq!(annotations.open_world_hint, Some(false));
}
