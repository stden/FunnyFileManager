//! Purpose: Verify server discovery precedence, language selection, and that
//! partial overrides merge the same way from YAML and from legacy JSON.

use super::*;
use tempfile::TempDir;

#[test]
fn batch_files_resolve_to_a_server_using_standard_io() {
    let root = TempDir::new().unwrap();
    let executable = root
        .path()
        .join(crate::lsp::program_file("batch-language-server"));
    crate::test_support::write_executable(&executable, b"batch server").unwrap();
    let probe = crate::lsp::Probe::new(None, Some(root.path().as_os_str().to_owned()));
    let settings = LspSettings::default();
    for filename in ["build.bat", "build.CMD"] {
        let (language, server) = settings.for_path(&root.path().join(filename)).unwrap();
        let availability = crate::lsp::availability(language, server, root.path(), &probe);
        assert_eq!(language, "bat");
        assert_eq!(
            availability.resolved(),
            Some(&crate::lsp::Resolved {
                executable: executable.clone(),
                args: vec!["--stdio".to_owned()],
            })
        );
    }
}

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
    settings.0.get_mut("rust").unwrap().enabled = Enablement::Disabled;
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

/// Each entry is merged over the catalog, so serde's own position points at
/// the end of the `lsp` mapping; the message names the entry that failed.
#[test]
fn a_deserialize_error_names_the_language_entry() {
    let error = serde_json::from_str::<LspSettings>(r#"{"python": {"args": "x"}}"#)
        .expect_err("args must be a list")
        .to_string();
    assert!(error.starts_with("python: invalid type: string"), "{error}");
    let error = serde_json::from_str::<LspSettings>(r#"{"go": ["gopls"]}"#)
        .expect_err("an entry must be a mapping")
        .to_string();
    assert!(
        error.starts_with("go: LSP settings must be an object"),
        "{error}"
    );
}

/// Read from the settings file, the reader's path names `lsp` and the entry
/// names the language, once each.
#[test]
fn a_settings_file_error_names_lsp_then_the_language() {
    let root = TempDir::new().unwrap();
    let path = root.path().join(crate::config::document::FILE_NAME);
    std::fs::write(&path, "lsp:\n  python:\n    args: x\n").unwrap();
    let error = crate::config::Settings::load_from(&path)
        .expect_err("args must be a list")
        .to_string();
    assert_eq!(
        error,
        "settings.yaml does not parse: lsp: python: invalid type: string \"x\", expected a sequence"
    );
}

/// The catalog is parsed once; every default is a copy of that one parse.
#[test]
fn the_catalog_is_parsed_once_and_is_the_default() {
    assert!(std::ptr::eq(LspSettings::catalog(), LspSettings::catalog()));
    assert_eq!(&LspSettings::default(), LspSettings::catalog());
    assert!(LspSettings::catalog().0.contains_key("python"));
}

/// `Enablement` keeps the file format: `enabled` is a boolean in YAML, and in
/// the legacy JSON file.
#[test]
fn enablement_keeps_enabled_a_boolean() {
    let settings: LspSettings = serde_norway::from_str("rust:\n  enabled: false\n").unwrap();
    assert_eq!(settings.0["rust"].enabled, Enablement::Disabled);
    let legacy: LspSettings = serde_json::from_str(r#"{"rust": {"enabled": false}}"#).unwrap();
    assert_eq!(legacy, settings);
    let value = serde_norway::to_value(&settings).unwrap();
    assert_eq!(value["rust"]["enabled"], serde_norway::Value::Bool(false));
    assert_eq!(value["python"]["enabled"], serde_norway::Value::Bool(true));
}
