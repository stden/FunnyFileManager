//! Unit tests: the cleanup rows under the home exist only when there is an
//! absolute home; an empty `HOME=` once put them under `.`, folders in the
//! panel's directory.

use super::{platform_caches, CleanupCandidate, CleanupKind, DiskCleanupDialog};
use crate::test_support::EnvVarGuard;

fn row_named<'a>(rows: &'a [CleanupCandidate], label: &str) -> &'a CleanupCandidate {
    rows.iter()
        .find(|row| row.label == label)
        .unwrap_or_else(|| panic!("no {label} row"))
}

fn assert_cache_row(rows: &[CleanupCandidate], label: &str, path: &std::path::Path) {
    let row = row_named(rows, label);
    assert_eq!(
        row.kind,
        CleanupKind::Path,
        "{label} must delete its folder"
    );
    assert_eq!(
        row.path.as_path(),
        path,
        "{label} must use the cache directory"
    );
}

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
    // HOME takes precedence over USERPROFILE on Windows too.
    let _home = EnvVarGuard::set("HOME", home.path());
    let dialog = DiskCleanupDialog::new();
    let heap_dumps = dialog
        .candidates
        .iter()
        .find(|row| row.label == "JVM heap dumps")
        .expect("the heap dumps row");
    assert_eq!(heap_dumps.path, home.path());
    assert_eq!(heap_dumps.kind, CleanupKind::HeapDumps);
    assert_eq!(
        dialog.candidates.first().map(|row| row.label),
        Some("Temp files")
    );
}

#[cfg(not(any(windows, target_os = "macos")))]
#[test]
fn unix_cache_rows_live_under_the_platform_cache_dir() {
    let base = tempfile::TempDir::new().expect("tempdir");
    let _lock = crate::test_support::env_lock();
    let caches = std::env::var_os("XDG_CACHE_HOME")
        .map(std::path::PathBuf::from)
        .filter(|path| path.is_absolute())
        .unwrap_or_else(|| base.path().join(".cache"));
    let rows = platform_caches(Some(base.path()));
    assert_cache_row(&rows, "JetBrains caches", &caches.join("JetBrains"));
    assert_cache_row(&rows, "Playwright browsers", &caches.join("ms-playwright"));
    assert_cache_row(&rows, "pip cache", &caches.join("pip"));
}

#[cfg(target_os = "macos")]
#[test]
fn macos_cache_rows_live_under_home_library() {
    let home = tempfile::TempDir::new().expect("tempdir");
    let caches = home.path().join("Library").join("Caches");
    let rows = platform_caches(Some(home.path()));
    assert_cache_row(&rows, "JetBrains caches", &caches.join("JetBrains"));
    assert_cache_row(&rows, "Playwright browsers", &caches.join("ms-playwright"));
    assert_cache_row(&rows, "pip cache", &caches.join("pip"));
}

#[cfg(windows)]
#[test]
fn windows_cache_rows_live_under_the_platform_cache_dir() {
    let home = tempfile::TempDir::new().expect("tempdir");
    let _lock = crate::test_support::env_lock();
    let local = std::env::var_os("LOCALAPPDATA").map_or_else(
        || home.path().join("AppData").join("Local"),
        std::path::PathBuf::from,
    );
    let rows = platform_caches(Some(home.path()));
    assert_cache_row(&rows, "JetBrains caches", &local.join("JetBrains"));
    assert_cache_row(&rows, "Docker WSL disk", &local.join("Docker").join("wsl"));
    assert_cache_row(&rows, "Playwright browsers", &local.join("ms-playwright"));
    assert_cache_row(&rows, "pip cache", &local.join("pip").join("Cache"));
}
