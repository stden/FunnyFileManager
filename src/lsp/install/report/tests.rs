//! Purpose: Verify availability states and launch parameters without package installation or server execution.

use super::*;
use crate::lsp::install::{fixtures::Packages, tests::simple_recipe};
use std::sync::atomic::AtomicBool;
use tempfile::TempDir;

/// An absolute command is taken as it is: found when it is runnable, and
/// with the configured arguments, whatever `PATH` holds.
#[test]
fn an_absolute_command_is_configured_with_its_own_arguments() {
    let root = TempDir::new().unwrap();
    let packages = Packages::new();
    let executable = root.path().join("server");
    crate::test_support::write_executable(&executable, b"local").unwrap();
    let server = ServerSettings {
        command: executable.to_string_lossy().into_owned(),
        args: vec!["--stdio".into()],
        ..ServerSettings::default()
    };
    assert_eq!(
        resolve(&server, None, root.path(), &packages),
        Availability::Configured(Resolved {
            executable,
            args: vec!["--stdio".into()],
        })
    );
    let missing = ServerSettings {
        command: root.path().join("gone").to_string_lossy().into_owned(),
        ..ServerSettings::default()
    };
    assert_eq!(
        resolve(&missing, None, root.path(), &packages),
        Availability::NotInstalled(NotInstalled::AutoInstallOff)
    );
    assert!(packages.calls.borrow().is_empty());
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
        args: vec!["--stdio".into()],
        ..ServerSettings::default()
    };
    assert_eq!(
        resolve(&server, None, root.path(), &packages),
        Availability::Configured(Resolved {
            executable,
            args: vec!["--stdio".into()],
        })
    );
    assert!(packages.calls.borrow().is_empty());
}

#[test]
fn a_cached_install_resolves_to_its_program_and_arguments() {
    let root = TempDir::new().unwrap();
    let packages = Packages::new();
    let settings = simple_recipe();
    let prepared =
        storage::prepare(&settings, root.path(), root.path(), &AtomicBool::new(false)).unwrap();
    assert_eq!(
        resolve(&settings, Some(root.path()), root.path(), &packages),
        Availability::Cached(Resolved {
            executable: PathBuf::from(&prepared.command),
            args: prepared.args.clone(),
        })
    );
    let receipt = Path::new(&prepared.command)
        .parent()
        .unwrap()
        .join("ready.json");
    fs::write(receipt, "invalid").unwrap();
    assert_eq!(
        resolve(&settings, Some(root.path()), root.path(), &packages),
        Availability::Failed(InstallError::Recipe)
    );
}

#[test]
fn no_cache_and_no_recipe_are_not_installed() {
    let root = TempDir::new().unwrap();
    let packages = Packages::new();
    assert_eq!(
        resolve(&simple_recipe(), None, root.path(), &packages),
        Availability::NotInstalled(NotInstalled::NoCache)
    );
    let no_recipe = ServerSettings::default();
    assert_eq!(
        resolve(&no_recipe, Some(root.path()), root.path(), &packages),
        Availability::NotInstalled(NotInstalled::AutoInstallOff)
    );
}

fn installed_java(
    root: &Path,
    workspace: &Path,
    packages: &Packages,
) -> (ServerSettings, ServerSettings) {
    let mut server = LspSettings::default().0.remove("java").unwrap();
    server.install = Some(InstallRecipe::Jdtls {
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
    let project = super::workspace(&server, &workspace.join("child/.."));
    let found = resolve(&server, Some(root.path()), &project, &moved);
    let resolved = found.resolved().expect("the cached server");
    assert_eq!(resolved.args.last(), prepared.args.last());
    assert_eq!(
        resolved.executable,
        moved
            .root
            .path()
            .join(super::super::launch::executable("java"))
    );
    assert!(moved.calls.borrow().is_empty());
}

#[path = "tests_availability.rs"]
mod probed;
