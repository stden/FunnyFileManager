//! Purpose: Exercise complete Python, npm and Java installation plans with deterministic package managers.

use super::{fixtures::Packages, storage, InstallError};
use crate::config::{InstallRecipe, LspSettings, ServerSettings};
use std::{fs, path::Path, sync::atomic::AtomicBool};
use tempfile::TempDir;

fn prepare(
    settings: &ServerSettings,
    root: &Path,
    workspace: &Path,
    packages: &Packages,
) -> Result<ServerSettings, InstallError> {
    storage::prepare_with(settings, root, workspace, &AtomicBool::new(false), packages)
}

#[test]
fn python_is_isolated_relocatable_and_cached_without_reinstalling() {
    let root = TempDir::new().unwrap();
    let packages = Packages::new();
    let settings = LspSettings::default().0.remove("python").unwrap();
    let prepared = prepare(&settings, root.path(), root.path(), &packages).unwrap();
    assert!(Path::new(&prepared.command).starts_with(root.path()));
    assert_eq!(prepared.args, ["-m", "pylsp"]);
    assert!(!prepared.command.contains("install-"));
    assert_eq!(packages.calls.borrow().len(), 2);
    assert!(packages.calls.borrow()[1].contains(&"python-lsp-server[pyflakes,pycodestyle]".into()));
    assert_eq!(
        prepared,
        prepare(&settings, root.path(), root.path(), &packages).unwrap()
    );
    assert_eq!(packages.calls.borrow().len(), 2);
}

fn npm_settings() -> ServerSettings {
    ServerSettings {
        args: vec!["--stdio".into()],
        install: Some(InstallRecipe::Npm {
            packages: vec!["server".into()],
            package: "server".into(),
            binary: "server".into(),
        }),
        ..ServerSettings::default()
    }
}

#[test]
fn npm_resolves_the_package_entry_point_and_passes_stdio_literally() {
    let root = TempDir::new().unwrap();
    let packages = Packages::new();
    let prepared = prepare(&npm_settings(), root.path(), root.path(), &packages).unwrap();
    assert!(Path::new(&prepared.command).is_file());
    assert!(Path::new(&prepared.args[0]).is_file());
    assert!(prepared.args[0].ends_with("main.js"));
    assert_eq!(prepared.args[1], "--stdio");
    assert!(packages.calls.borrow()[0].contains(&"--prefix".into()));
    assert_eq!(packages.calls.borrow().len(), 1);
}

fn java_settings(packages: &Packages) -> ServerSettings {
    ServerSettings {
        install: Some(InstallRecipe::Jdtls {
            url: "https://example.com/server.tar.gz".into(),
            sha256: storage::digest(&packages.archive),
        }),
        ..ServerSettings::default()
    }
}

#[test]
fn java_download_is_verified_and_each_workspace_has_its_own_data() {
    let root = TempDir::new().unwrap();
    let packages = Packages::new();
    let settings = java_settings(&packages);
    let first = prepare(&settings, root.path(), &root.path().join("one"), &packages).unwrap();
    let second = prepare(&settings, root.path(), &root.path().join("two"), &packages).unwrap();
    assert_ne!(first.args.last(), second.args.last());
    assert_eq!(first.command, second.command);
    let config = first
        .args
        .iter()
        .position(|arg| arg == "-configuration")
        .unwrap();
    assert_eq!(
        fs::read(Path::new(&first.args[config + 1]).join("config.ini")).unwrap(),
        b"data"
    );
    assert!(first.args.iter().any(|arg| Path::new(arg)
        .extension()
        .is_some_and(|ext| ext.eq_ignore_ascii_case("jar"))));
    assert_eq!(packages.calls.borrow().len(), 3);
}

#[test]
fn java_rejects_old_runtimes_before_downloading_anything() {
    let root = TempDir::new().unwrap();
    let mut packages = Packages::new();
    packages.version = "openjdk 17.0.1".into();
    assert!(matches!(
        prepare(
            &java_settings(&packages),
            root.path(),
            root.path(),
            &packages
        ),
        Err(InstallError::Missing(_))
    ));
    assert_eq!(packages.calls.borrow().len(), 1);
}

#[test]
fn java_rejects_changed_downloads_without_publishing_a_receipt() {
    let root = TempDir::new().unwrap();
    let mut packages = Packages::new();
    let settings = java_settings(&packages);
    packages.archive.push(1);
    assert_eq!(
        prepare(&settings, root.path(), root.path(), &packages).unwrap_err(),
        InstallError::Integrity
    );
    assert!(fs::read_dir(root.path())
        .unwrap()
        .all(|entry| entry.unwrap().path().is_file()));
}
