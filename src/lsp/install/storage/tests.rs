//! Purpose: Verify atomic cache publication, locking, recipe identities, and confined paths.

use super::*;
use crate::lsp::install::tests::{recipe, simple_recipe};
use std::sync::{atomic::Ordering, Arc};
use tempfile::TempDir;

#[test]
fn reusing_a_complete_install_does_not_run_its_installer_again() {
    let root = TempDir::new().unwrap();
    let stop = AtomicBool::new(false);
    let settings = recipe("from pathlib import Path; marker=Path('runs'); marker.write_text(marker.read_text()+'x' if marker.exists() else 'x'); Path('server').touch()");
    let first = prepare(&settings, root.path(), root.path(), &stop).unwrap();
    let second = prepare(&settings, root.path(), root.path(), &stop).unwrap();
    assert_eq!(first, second);
    let installed = Path::new(&first.command).parent().unwrap();
    assert_eq!(fs::read_to_string(installed.join("runs")).unwrap(), "x");
    assert!(installed.join("ready.json").is_file());
}

#[test]
fn a_corrupt_receipt_is_reported_without_overwriting_the_installed_server() {
    let root = TempDir::new().unwrap();
    let stop = AtomicBool::new(false);
    let settings = simple_recipe();
    let installed = prepare(&settings, root.path(), root.path(), &stop).unwrap();
    let receipt = Path::new(&installed.command)
        .parent()
        .unwrap()
        .join("ready.json");
    fs::write(receipt, "corrupt").unwrap();
    assert_eq!(
        prepare(&settings, root.path(), root.path(), &stop).unwrap_err(),
        InstallError::Recipe
    );
    assert_eq!(fs::read_to_string(installed.command).unwrap(), "installed");
}

#[test]
fn waiting_for_another_installer_is_cancellable_and_unlocks_on_drop() {
    let root = TempDir::new().unwrap();
    let path = root.path().join("package.lock");
    let held = lock(&path, &AtomicBool::new(false)).unwrap();
    let stop = Arc::new(AtomicBool::new(false));
    let cancelled = Arc::clone(&stop);
    let other = path.clone();
    let thread = std::thread::spawn(move || lock(&other, &cancelled));
    stop.store(true, Ordering::Release);
    assert!(matches!(
        thread.join().unwrap(),
        Err(InstallError::Cancelled)
    ));
    drop(held);
    assert!(lock(&path, &AtomicBool::new(false)).is_ok());
}

#[test]
fn recipe_paths_cannot_escape_the_installation_directory() {
    let root = TempDir::new().unwrap();
    for name in [
        "",
        "../outside",
        "/outside",
        "C:/outside",
        "..\\outside",
        "bin/../../outside",
    ] {
        assert_eq!(
            relative(root.path(), name).unwrap_err(),
            InstallError::Recipe
        );
    }
    assert_eq!(
        relative(root.path(), "bin/server").unwrap(),
        root.path().join("bin/server")
    );
    assert_eq!(
        digest(b"abc"),
        "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
    );
}

#[test]
fn invalid_storage_and_absent_recipes_fail_before_installation() {
    let root = TempDir::new().unwrap();
    let path = root.path().join("file");
    fs::write(&path, "kept").unwrap();
    let stop = AtomicBool::new(false);
    assert!(matches!(
        prepare(&simple_recipe(), &path, root.path(), &stop),
        Err(InstallError::Io(_))
    ));
    assert_eq!(
        prepare(&ServerSettings::default(), root.path(), root.path(), &stop).unwrap_err(),
        InstallError::Recipe
    );
    assert_eq!(fs::read_to_string(path).unwrap(), "kept");
}
