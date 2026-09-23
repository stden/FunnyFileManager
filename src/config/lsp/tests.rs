//! Purpose: Verify server discovery precedence, language selection, and that
//! partial overrides merge the same way from YAML and from legacy JSON.

use super::*;
use tempfile::TempDir;

#[test]
fn default_command_uses_path_and_explicit_configuration_is_preserved() {
    let root = TempDir::new().unwrap();
    let mut settings = LspSettings::default();
    let path = root.path().join("source.rs");
    assert_eq!(settings.for_path(&path).unwrap().1.command, "rust-analyzer");
    assert_eq!(
        settings.for_path(&root.path().join("source.Rs")).unwrap().0,
        "rust"
    );
    let custom = root
        .path()
        .join("server.exe")
        .to_string_lossy()
        .into_owned();
    settings
        .0
        .get_mut("rust")
        .unwrap()
        .command
        .clone_from(&custom);
    let encoded = serde_json::to_string(&settings).unwrap();
    let decoded: LspSettings = serde_json::from_str(&encoded).unwrap();
    assert_eq!(decoded.for_path(&path).unwrap().1.command, custom);
    settings.0.get_mut("rust").unwrap().enabled = false;
    assert!(settings.for_path(&path).is_none());
    assert!(settings.for_path(&root.path().join("notes.txt")).is_none());
    assert!(settings.for_path(root.path()).is_none());
}

#[test]
fn another_language_uses_its_own_extension_and_command() {
    let root = TempDir::new().unwrap();
    let mut settings = LspSettings::default();
    settings.0.insert(
        "python".into(),
        ServerSettings {
            command: "python-language-server".into(),
            extensions: vec!["py".into()],
            ..ServerSettings::default()
        },
    );
    let (language, server) = settings.for_path(&root.path().join("test.py")).unwrap();
    assert_eq!(language, "python");
    assert_eq!(server.command, "python-language-server");
}

#[test]
fn an_existing_rust_only_configuration_gains_other_language_defaults() {
    let settings: LspSettings =
        serde_json::from_str(r#"{"rust":{"command":"custom-rust"},"python":{"enabled":false}}"#)
            .unwrap();
    let root = TempDir::new().unwrap();
    assert_eq!(
        settings.for_path(&root.path().join("Main.java")).unwrap().0,
        "java"
    );
    assert_eq!(
        settings.for_path(&root.path().join("app.ts")).unwrap().0,
        "typescript"
    );
    assert!(settings.for_path(&root.path().join("app.py")).is_none());
    assert_eq!(
        settings
            .for_path(&root.path().join("app.rs"))
            .unwrap()
            .1
            .command,
        "custom-rust"
    );
}

#[test]
fn partial_language_overrides_retain_their_catalog_extensions_and_recipe() {
    let root = TempDir::new().unwrap();
    let settings: LspSettings =
        serde_json::from_str(r#"{"python":{"command":"custom-python"},"java":{"install":null}}"#)
            .unwrap();
    let (_, python) = settings.for_path(&root.path().join("main.py")).unwrap();
    assert_eq!(python.command, "custom-python");
    assert!(python.install.is_some());
    assert!(settings.0["java"].install.is_none());
}

/// The overrides the docs show, written as a person writes YAML: a partial
/// entry keeps its catalog fields, `install: null` turns auto-install off,
/// and an `env` value that starts with `{` must be quoted.
#[test]
fn partial_overrides_written_in_yaml_merge_onto_the_catalog() {
    let root = TempDir::new().unwrap();
    let yaml = "python:\n  enabled: false\njava:\n  install: null\ngo:\n  command: gopls\n  extensions: [go]\n  install:\n    kind: command\n    executable: '{install_dir}/bin/gopls'\n    steps:\n      - command: go\n        args: [install, golang.org/x/tools/gopls@latest]\n        env:\n          GOBIN: '{install_dir}/bin'\n";
    let settings: LspSettings = serde_norway::from_str(yaml).unwrap();
    assert!(settings.for_path(&root.path().join("app.py")).is_none());
    assert!(
        settings.0["python"].install.is_some(),
        "catalog fields stay"
    );
    assert!(settings.0["java"].install.is_none());
    let (_, go) = settings.for_path(&root.path().join("main.go")).unwrap();
    let Some(InstallRecipe::Command { steps, executable }) = &go.install else {
        panic!("a command recipe: {:?}", go.install);
    };
    assert_eq!(executable, "{install_dir}/bin/gopls");
    assert_eq!(steps[0].env["GOBIN"], "{install_dir}/bin");
    let unquoted = yaml.replace("'{install_dir}/bin'", "{install_dir}/bin");
    assert!(serde_norway::from_str::<LspSettings>(&unquoted).is_err());
}
