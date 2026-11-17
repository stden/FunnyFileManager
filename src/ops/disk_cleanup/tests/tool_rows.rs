//! Purpose: The tool rows of the Disk Cleanup dialog, measured and cleaned the
//! way `:cleanup` does. Each row points at its tool's folder through the
//! variable the tool reads, and removes only what the row names: the files
//! beside it, the artifacts and the other folders all survive.

use std::fs;
use std::path::{Path, PathBuf};

use tempfile::TempDir;

use crate::models::{App, CleanupCandidate, CleanupKind, DiskCleanupDialog};
use crate::test_support::disk_cleanup::{app, run};
use crate::test_support::EnvVarsGuard;

mod gradle;
mod maven;

/// The dialog's own row `label`, measured as `:cleanup` measures it.
pub(super) fn measured_row(label: &str) -> CleanupCandidate {
    let mut dialog = DiskCleanupDialog::new();
    dialog.candidates.retain(|row| row.label == label);
    crate::ops::disk_cleanup::measure(&mut dialog, Path::new("/"));
    dialog
        .candidates
        .pop()
        .unwrap_or_else(|| panic!("the dialog has no row named {label}"))
}

/// Cleans `rows` as the dialog does once they are ticked and run.
pub(super) fn clean(rows: Vec<CleanupCandidate>) -> App {
    let temp = TempDir::new().unwrap();
    let mut app = app(temp.path(), rows);
    run(&mut app);
    app
}

pub(super) fn write(path: &Path, bytes: &[u8]) {
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, bytes).unwrap();
}

pub(super) fn assert_kept(path: &Path, bytes: &[u8]) {
    assert_eq!(
        fs::read(path).unwrap(),
        bytes,
        "{} was touched",
        path.display()
    );
}

fn is_empty_dir(dir: &Path) -> bool {
    fs::read_dir(dir).unwrap().next().is_none()
}

#[test]
fn rustup_temp_files_empty_the_tmp_folder_and_keep_the_toolchains() {
    let temp = TempDir::new().unwrap();
    let rustup = temp.path().join("rustup");
    write(&rustup.join("tmp/partial.part"), b"12");
    write(&rustup.join("tmp/nested/data.bin"), b"345");
    write(&rustup.join("settings.toml"), b"keep");
    write(&rustup.join("toolchains/stable/bin/rustc"), b"keep");
    let _env = EnvVarsGuard::set(&[("RUSTUP_HOME", rustup.as_path())]);

    let row = measured_row("Rustup temp files");

    assert_eq!(row.kind, CleanupKind::Contents);
    assert_eq!(row.path, rustup.join("tmp"));
    assert!(row.is_checked(), "temporary files start ticked");
    let app = clean(vec![row]);
    assert!(is_empty_dir(&rustup.join("tmp")), "the folder is emptied");
    assert!(rustup.join("tmp").is_dir(), "and the folder itself stays");
    assert_kept(&rustup.join("settings.toml"), b"keep");
    assert_kept(&rustup.join("toolchains/stable/bin/rustc"), b"keep");
    assert_eq!(app.status.to_string(), "Freed 5 B");
}

#[test]
fn rustup_downloads_start_unticked_and_remove_only_the_download_cache() {
    let temp = TempDir::new().unwrap();
    let rustup = temp.path().join("rustup");
    write(&rustup.join("downloads/archive.tar.xz"), b"cache");
    write(&rustup.join("tmp/keep.part"), b"keep");
    write(&rustup.join("settings.toml"), b"keep");
    let _env = EnvVarsGuard::set(&[("RUSTUP_HOME", rustup.as_path())]);

    let mut row = measured_row("Rustup downloads");

    assert_eq!(row.kind, CleanupKind::Path);
    assert_eq!(row.path, rustup.join("downloads"));
    assert!(
        !row.is_checked(),
        "a download is fetched again, so it starts unticked"
    );
    row.toggle_checked();
    let app = clean(vec![row]);
    assert!(!rustup.join("downloads").exists());
    assert_kept(&rustup.join("tmp/keep.part"), b"keep");
    assert_kept(&rustup.join("settings.toml"), b"keep");
    assert_eq!(app.status.to_string(), "Freed 5 B");
}

#[test]
fn cargo_package_cache_lock_is_removed_and_the_registry_stays() {
    let temp = TempDir::new().unwrap();
    let cargo = temp.path().join("cargo");
    write(&cargo.join(".package-cache"), b"");
    write(&cargo.join("registry/index/entry"), b"keep");
    write(&cargo.join("bin/tool"), b"keep");
    let _env = EnvVarsGuard::set(&[("CARGO_HOME", cargo.as_path())]);

    let mut row = measured_row("Cargo package-cache lock");

    assert_eq!(row.kind, CleanupKind::Path);
    assert_eq!(row.path, cargo.join(".package-cache"));
    assert!(
        !row.is_checked(),
        "a running cargo holds this lock, so it starts unticked"
    );
    row.toggle_checked();
    clean(vec![row]);
    assert!(!cargo.join(".package-cache").exists());
    assert_kept(&cargo.join("registry/index/entry"), b"keep");
    assert_kept(&cargo.join("bin/tool"), b"keep");
}

#[test]
fn sccache_dir_names_the_compiler_cache_and_its_neighbours_stay() {
    let temp = TempDir::new().unwrap();
    let sccache: PathBuf = temp.path().join("custom-sccache");
    write(&sccache.join("entry"), b"cache");
    write(&temp.path().join("keep.txt"), b"keep");
    let _env = EnvVarsGuard::set(&[
        ("SCCACHE_DIR", sccache.as_path()),
        ("XDG_CACHE_HOME", temp.path()),
    ]);

    let mut row = measured_row("sccache cache");

    assert_eq!(row.kind, CleanupKind::Path);
    assert_eq!(row.path, sccache);
    assert!(
        !row.is_checked(),
        "the compiler cache is rebuilt, so it starts unticked"
    );
    row.toggle_checked();
    let app = clean(vec![row]);
    assert!(!sccache.exists());
    assert_kept(&temp.path().join("keep.txt"), b"keep");
    assert_eq!(app.status.to_string(), "Freed 5 B");
}

#[cfg(all(unix, not(target_os = "macos")))]
#[test]
fn sccache_without_a_variable_is_the_sccache_folder_of_the_xdg_cache() {
    let temp = TempDir::new().unwrap();
    let cache = temp.path().join("cache");
    write(&cache.join("sccache/entry"), b"cache");
    write(&cache.join("pip/wheel.whl"), b"keep");
    let _env = EnvVarsGuard::set(&[
        ("SCCACHE_DIR", Path::new("")),
        ("XDG_CACHE_HOME", cache.as_path()),
    ]);

    let mut row = measured_row("sccache cache");
    assert_eq!(row.path, cache.join("sccache"));
    row.toggle_checked();
    clean(vec![row]);

    assert!(!cache.join("sccache").exists());
    assert_kept(&cache.join("pip/wheel.whl"), b"keep");
}
