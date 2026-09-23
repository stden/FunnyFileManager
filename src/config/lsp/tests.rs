//! Purpose: Verify server discovery precedence, language selection, and JSON compatibility.

use super::*;
use tempfile::TempDir;

#[test]
fn default_command_uses_path_and_explicit_configuration_is_preserved() {
    let root = TempDir::new().unwrap();
    let mut settings = LspSettings::default();
    let path = root.path().join("source.rs");
    assert_eq!(settings.for_path(&path).unwrap().1.command, "rust-analyzer");
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
