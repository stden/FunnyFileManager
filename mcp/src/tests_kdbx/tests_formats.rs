//! Purpose: The `kbdx` tool's JSON, YAML and dotenv exports, and the schema it
//! advertises.

use super::*;
use serde_json::json;
use std::io::Write as _;

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

    let annotations = tool.annotations.expect("tool annotations");
    assert_eq!(annotations.read_only_hint, Some(false));
    assert_eq!(annotations.destructive_hint, Some(false));
    assert_eq!(annotations.open_world_hint, Some(false));
}
