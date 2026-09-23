//! Purpose: Verify relocatable launch receipts and runtime version parsing.

use super::*;
use crate::lsp::install::environment::System;
use tempfile::TempDir;

#[test]
fn launch_receipts_are_relocated_and_keep_literal_server_arguments() {
    let root = TempDir::new().unwrap();
    crate::test_support::write_executable(&root.path().join("server"), b"binary").unwrap();
    let recipe = InstallRecipe::Command {
        steps: vec![],
        executable: "server".into(),
    };
    let launch = for_recipe(&recipe, root.path(), &AtomicBool::new(false), &System).unwrap();
    let settings = ServerSettings {
        args: vec!["--value=a & b".into()],
        ..ServerSettings::default()
    };
    let prepared = launch
        .settings(&settings, root.path(), root.path(), &System)
        .unwrap();
    assert_eq!(Path::new(&prepared.command), root.path().join("server"));
    assert_eq!(prepared.args, ["--value=a & b"]);
    assert!(prepared.install.is_none());
    fs::remove_file(root.path().join("server")).unwrap();
    assert!(launch
        .settings(&settings, root.path(), root.path(), &System)
        .is_err());
}

#[test]
fn java_versions_distinguish_current_runtimes_from_java_eight_and_invalid_output() {
    assert_eq!(java_major("openjdk 21.0.11 2026-04-21"), Some(21));
    assert_eq!(java_major("java 25-ea"), Some(25));
    assert_eq!(java_major("java 1.8.0"), Some(1));
    assert_eq!(java_major("unrecognized"), None);
    assert_eq!(java_major("java unknown"), None);
}

#[test]
fn a_directory_named_like_a_java_launcher_is_rejected() {
    let root = TempDir::new().unwrap();
    fs::create_dir_all(
        root.path()
            .join("plugins/org.eclipse.equinox.launcher_1.jar"),
    )
    .unwrap();
    assert_eq!(
        java_launcher(root.path()).unwrap_err(),
        InstallError::Recipe
    );
}

#[cfg(unix)]
#[test]
fn custom_servers_require_executable_permission() {
    use std::os::unix::fs::PermissionsExt;
    let root = TempDir::new().unwrap();
    let path = root.path().join("server");
    fs::write(&path, "binary").unwrap();
    fs::set_permissions(&path, fs::Permissions::from_mode(0o644)).unwrap();
    let recipe = InstallRecipe::Command {
        steps: vec![],
        executable: "server".into(),
    };
    assert!(matches!(
        for_recipe(&recipe, root.path(), &AtomicBool::new(false), &System),
        Err(InstallError::Recipe)
    ));
}
