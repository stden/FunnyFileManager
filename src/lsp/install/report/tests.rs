//! Purpose: Verify inventory states and parameters without package installation or server execution.

use super::*;
use crate::lsp::install::{fixtures::Packages, tests::simple_recipe};
use std::sync::atomic::AtomicBool;
use tempfile::TempDir;

#[test]
fn inventory_distinguishes_disabled_missing_and_configured_servers() {
    let root = TempDir::new().unwrap();
    let packages = Packages::new();
    let mut settings = LspSettings::default();
    settings.0.get_mut("python").unwrap().enabled = false;
    settings.0.get_mut("rust").unwrap().command = packages
        .root
        .path()
        .join(super::super::launch::executable("java"))
        .to_string_lossy()
        .into_owned();
    let lines = report(&settings, root.path(), Some(root.path()), &packages).join("\n");
    assert!(lines.contains("python [disabled] .py .pyi"));
    assert!(lines.contains("found configured executable"));
    assert!(lines.contains("not installed; installs after workspace trust"));
    assert!(lines.contains("Configured argument: --stdio"));
    assert!(lines.contains("python-lsp-server"));
    assert!(packages.calls.borrow().is_empty());
    assert_eq!(fs::read_dir(root.path()).unwrap().count(), 0);
}

#[test]
fn inventory_names_the_settings_and_passes_through_local_launch_parameters() {
    let root = TempDir::new().unwrap();
    let executable = root.path().join("server");
    crate::test_support::write_executable(&executable, b"local").unwrap();
    let mut settings = LspSettings(std::collections::BTreeMap::new());
    settings.0.insert(
        "local".into(),
        ServerSettings {
            command: executable.to_string_lossy().into_owned(),
            args: vec!["--stdio".into()],
            ..ServerSettings::default()
        },
    );
    let text = inventory(&settings, root.path()).join("\n");
    assert!(text.contains("local [enabled]"));
    assert!(text.contains("Launch argument: --stdio"));
    assert!(text.contains(&*executable.to_string_lossy()));
    assert!(text.contains("Auto-install: disabled"));
}

#[test]
fn relative_commands_are_resolved_from_the_project_root() {
    let root = TempDir::new().unwrap();
    let packages = Packages::new();
    let executable = root.path().join("tools/server");
    fs::create_dir(executable.parent().unwrap()).unwrap();
    crate::test_support::write_executable(&executable, b"server").unwrap();
    let server = ServerSettings {
        command: "./tools/server".into(),
        ..ServerSettings::default()
    };
    let (_, prepared) = resolve(&server, None, root.path(), &packages).unwrap();
    assert_eq!(Path::new(&prepared.command), executable);
    assert!(packages.calls.borrow().is_empty());
}

#[test]
fn inventory_shows_cached_arguments_and_detects_corrupt_receipts() {
    let root = TempDir::new().unwrap();
    let packages = Packages::new();
    let settings = simple_recipe();
    let prepared =
        storage::prepare(&settings, root.path(), root.path(), &AtomicBool::new(false)).unwrap();
    let (state, reported) = resolve(&settings, Some(root.path()), root.path(), &packages).unwrap();
    assert_eq!(state, "installed in FFM cache");
    assert_eq!(reported, prepared);
    fs::write(
        Path::new(&prepared.command)
            .parent()
            .unwrap()
            .join("ready.json"),
        "invalid",
    )
    .unwrap();
    assert!(matches!(
        resolve(&settings, Some(root.path()), root.path(), &packages),
        Err(InstallError::Recipe)
    ));
    assert!(resolve(&settings, None, root.path(), &packages).is_err());
    assert!(resolve(
        &ServerSettings::default(),
        Some(root.path()),
        root.path(),
        &packages
    )
    .is_err());
}

fn installed_java(
    root: &Path,
    workspace: &Path,
    packages: &Packages,
) -> (ServerSettings, ServerSettings) {
    let mut server = LspSettings::default().0.remove("java").unwrap();
    server.install = Some(crate::config::InstallRecipe::Jdtls {
        url: "https://example.com/server.tar.gz".into(),
        sha256: storage::digest(&packages.archive),
    });
    let prepared = storage::prepare_with(
        &server,
        root,
        &workspace.canonicalize().unwrap(),
        &AtomicBool::new(false),
        packages,
    )
    .unwrap();
    (server, prepared)
}

#[test]
fn java_inventory_uses_the_current_runtime_and_canonical_workspace_without_execution() {
    let root = TempDir::new().unwrap();
    let first = Packages::new();
    let moved = Packages::new();
    let workspace = root.path().join("project");
    fs::create_dir_all(workspace.join("child")).unwrap();
    let (server, prepared) = installed_java(root.path(), &workspace, &first);
    fs::remove_file(&prepared.command).unwrap();
    let mut lines = Vec::new();
    availability(
        &mut lines,
        &server,
        &workspace.join("child/.."),
        Some(root.path()),
        &moved,
    );
    assert!(lines.contains(&format!(
        "  Launch argument: {}",
        prepared.args.last().unwrap()
    )));
    assert!(lines.contains(&format!(
        "  Executable: {}",
        moved
            .root
            .path()
            .join(super::super::launch::executable("java"))
            .display()
    )));
    assert!(moved.calls.borrow().is_empty());
}
