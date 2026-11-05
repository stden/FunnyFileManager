//! Unit tests: the cleanup rows under the home exist only when there is an
//! absolute home; an empty `HOME=` once put them under `.`, folders in the
//! panel's directory. Each tool row reads the variable its tool reads, and
//! falls back to the folder the tool creates under the home.

use std::path::Path;

use super::{CleanupCandidate, CleanupKind, DiskCleanupDialog, TempScope};
use crate::test_support::{EnvVarGuard, EnvVarsGuard};
use tempfile::TempDir;

#[cfg(unix)]
#[test]
fn without_an_absolute_home_no_row_names_a_folder_under_it() {
    let _env = EnvVarsGuard::set(&[
        ("HOME", Path::new("")),
        ("XDG_CACHE_HOME", Path::new("")),
        ("GRADLE_USER_HOME", Path::new("")),
        ("RUSTUP_HOME", Path::new("")),
        ("CARGO_HOME", Path::new("")),
        ("SCCACHE_DIR", Path::new("")),
    ]);
    let dialog = DiskCleanupDialog::new();
    let labels: Vec<&str> = dialog.candidates.iter().map(|row| row.label).collect();
    for label in [
        "Gradle caches",
        "npm cache",
        "Gradle wrapper cache",
        "JVM heap dumps",
        "Rustup temp files",
        "Rustup downloads",
        "Cargo package-cache lock",
        "sccache cache",
        "Gradle daemon logs",
        "Gradle transforms",
        "Maven partial downloads",
    ] {
        assert!(!labels.contains(&label), "{label} in {labels:?}");
    }
    assert!(dialog
        .candidates
        .iter()
        .all(|row| row.path.is_absolute() || row.path == Path::new("target")));
}

#[test]
fn with_a_home_every_row_is_offered() {
    let home = TempDir::new().expect("tempdir");
    let _home = EnvVarGuard::set(
        if cfg!(windows) { "USERPROFILE" } else { "HOME" },
        home.path(),
    );
    let dialog = DiskCleanupDialog::new();
    let heap_dumps = dialog
        .candidates
        .iter()
        .find(|row| row.label == "JVM heap dumps")
        .expect("the heap dumps row");
    assert_eq!(heap_dumps.path, home.path());
    assert_eq!(
        dialog.candidates.first().map(|row| row.label),
        Some("Temp files")
    );
}

/// The row named `label`, as the dialog offers it.
pub(super) fn row<'a>(dialog: &'a DiskCleanupDialog, label: &str) -> &'a CleanupCandidate {
    dialog
        .candidates
        .iter()
        .find(|row| row.label == label)
        .unwrap_or_else(|| panic!("no row named {label}"))
}

#[test]
fn the_rust_rows_read_the_variables_their_tools_read() {
    let tools = TempDir::new().expect("tempdir");
    let rustup = tools.path().join("rustup");
    let cargo = tools.path().join("cargo");
    let sccache = tools.path().join("sccache");
    let _env = EnvVarsGuard::set(&[
        ("RUSTUP_HOME", &rustup),
        ("CARGO_HOME", &cargo),
        ("SCCACHE_DIR", &sccache),
    ]);

    let dialog = DiskCleanupDialog::new();

    let tmp = row(&dialog, "Rustup temp files");
    assert_eq!(tmp.kind, CleanupKind::Contents);
    assert_eq!(tmp.path, rustup.join("tmp"));
    let downloads = row(&dialog, "Rustup downloads");
    assert_eq!(downloads.kind, CleanupKind::Path);
    assert_eq!(downloads.path, rustup.join("downloads"));
    let lock = row(&dialog, "Cargo package-cache lock");
    assert_eq!(lock.kind, CleanupKind::Path);
    assert_eq!(lock.path, cargo.join(".package-cache"));
    assert_eq!(row(&dialog, "sccache cache").path, sccache);
}

#[test]
fn the_gradle_and_maven_rows_read_the_folders_their_tools_read() {
    let home = TempDir::new().expect("tempdir");
    let tools = TempDir::new().expect("tempdir");
    let gradle = tools.path().join("gradle");
    let _env = EnvVarsGuard::set(&[("HOME", home.path()), ("GRADLE_USER_HOME", &gradle)]);

    let dialog = DiskCleanupDialog::new();

    let logs = row(&dialog, "Gradle daemon logs");
    assert_eq!(logs.kind, CleanupKind::Temp(TempScope::GradleDaemonLogs));
    assert_eq!(logs.path, gradle.join("daemon"));
    let transforms = row(&dialog, "Gradle transforms");
    assert_eq!(
        transforms.kind,
        CleanupKind::Temp(TempScope::GradleTransforms)
    );
    assert_eq!(transforms.path, gradle.join("caches"));
    assert_eq!(row(&dialog, "Gradle caches").path, gradle.join("caches"));
    assert_eq!(
        row(&dialog, "Gradle wrapper cache").path,
        gradle.join("wrapper")
    );
    let maven = row(&dialog, "Maven partial downloads");
    assert_eq!(
        maven.kind,
        CleanupKind::Temp(TempScope::MavenPartialDownloads)
    );
    assert_eq!(maven.path, home.path().join(".m2").join("repository"));
}

/// A variable that is empty or relative is not a folder: the tool's own
/// default under the home is what its row names.
#[test]
fn a_tool_variable_that_is_not_absolute_falls_back_to_the_default_folder() {
    let home = TempDir::new().expect("tempdir");
    let _env = EnvVarsGuard::set(&[
        ("HOME", home.path()),
        ("RUSTUP_HOME", Path::new("relative/rustup")),
        ("CARGO_HOME", Path::new("")),
        ("GRADLE_USER_HOME", Path::new("relative/gradle")),
        ("SCCACHE_DIR", Path::new("relative/sccache")),
    ]);

    let dialog = DiskCleanupDialog::new();

    assert_eq!(
        row(&dialog, "Rustup downloads").path,
        home.path().join(".rustup").join("downloads")
    );
    assert_eq!(
        row(&dialog, "Cargo package-cache lock").path,
        home.path().join(".cargo").join(".package-cache")
    );
    assert_eq!(
        row(&dialog, "Gradle daemon logs").path,
        home.path().join(".gradle").join("daemon")
    );
    assert!(
        row(&dialog, "sccache cache").path.is_absolute(),
        "sccache falls back to the cache folder under the home"
    );
}

#[test]
fn a_row_starts_ticked_only_when_what_it_removes_is_temporary() {
    assert!(CleanupKind::Contents.starts_checked());
    assert!(CleanupKind::HeapDumps.starts_checked());
    assert!(!CleanupKind::Path.starts_checked());
    assert!(CleanupKind::Temp(TempScope::MavenPartialDownloads).starts_checked());
    assert!(CleanupKind::Temp(TempScope::GradleDaemonLogs).starts_checked());
    assert!(!CleanupKind::Temp(TempScope::GradleTransforms).starts_checked());
}
