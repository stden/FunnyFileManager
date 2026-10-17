//! Purpose: Verify the workspace trust policy is explicit, validated, and preserved by settings saves.

use super::*;

#[test]
fn automatic_lsp_trust_loads_and_survives_an_unrelated_settings_save() {
    let root = TempDir::new().unwrap();
    let path = root.path().join("settings.yaml");
    std::fs::write(&path, "editor: {lsp_trust: automatic, tab_size: 3}\n").unwrap();
    let mut settings = Settings::load_from(&path).unwrap();
    assert_eq!(
        serde_json::to_value(&settings).unwrap()["editor"]["lsp_trust"],
        "automatic"
    );
    settings.system.history_size = 7;
    settings.save_to(&path).unwrap();
    let saved = Settings::load_from(&path).unwrap();
    assert_eq!(saved, settings);
    assert_eq!(saved.editor.tab_size, 3);
}

#[test]
fn invalid_lsp_trust_is_reported_instead_of_being_ignored() {
    let root = TempDir::new().unwrap();
    let path = root.path().join("settings.yaml");
    std::fs::write(&path, "editor: {lsp_trust: always}\n").unwrap();
    let error = Settings::load_from(&path).expect_err("unknown trust policy");
    assert!(error.to_string().contains("lsp_trust"));
}

#[test]
fn default_settings_require_explicit_workspace_approval() {
    assert_eq!(
        serde_json::to_value(Settings::default()).unwrap()["editor"]["lsp_trust"],
        "prompt"
    );
}
