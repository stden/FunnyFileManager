//! Unit tests: the cleanup rows under the home exist only when there is an
//! absolute home; an empty `HOME=` once put them under `.`, folders in the
//! panel's directory.

use super::DiskCleanupDialog;
use crate::test_support::EnvVarGuard;

#[cfg(unix)]
#[test]
fn without_an_absolute_home_no_row_names_a_folder_under_it() {
    let _home = EnvVarGuard::set("HOME", std::path::Path::new(""));
    let dialog = DiskCleanupDialog::new();
    let labels: Vec<&str> = dialog.candidates.iter().map(|row| row.label).collect();
    for label in [
        "Gradle caches",
        "npm cache",
        "Gradle wrapper cache",
        "JVM heap dumps",
    ] {
        assert!(!labels.contains(&label), "{label} in {labels:?}");
    }
    assert!(dialog
        .candidates
        .iter()
        .all(|row| row.path.is_absolute() || row.path == std::path::Path::new("target")));
}

#[test]
fn with_a_home_every_row_is_offered() {
    let home = tempfile::TempDir::new().expect("tempdir");
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
