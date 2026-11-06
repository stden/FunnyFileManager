//! Unit tests on Windows: the cache rows live under `%LOCALAPPDATA%`, and
//! without it under `AppData\Local` in the home. These rows delete folders,
//! so each one is pinned to the exact folder it names.

use std::path::Path;

use super::tests::row;
use super::{CleanupKind, DiskCleanupDialog};
use crate::test_support::EnvVarsGuard;
use tempfile::TempDir;

#[test]
fn windows_cache_rows_live_under_localappdata() {
    let home = TempDir::new().expect("tempdir");
    let local = home.path().join("Local");
    let _env = EnvVarsGuard::set(&[("HOME", home.path()), ("LOCALAPPDATA", local.as_path())]);
    let dialog = DiskCleanupDialog::new();
    for (label, folder) in [
        ("JetBrains caches", local.join("JetBrains")),
        ("Docker WSL disk", local.join("Docker").join("wsl")),
        ("Playwright browsers", local.join("ms-playwright")),
        ("pip cache", local.join("pip").join("Cache")),
    ] {
        let found = row(&dialog, label);
        assert_eq!(found.path, folder, "{label}");
        assert_eq!(found.kind, CleanupKind::Path, "{label}");
    }
}

#[test]
fn without_localappdata_the_cache_rows_fall_back_to_appdata_local() {
    let home = TempDir::new().expect("tempdir");
    let _env = EnvVarsGuard::set(&[("HOME", home.path()), ("LOCALAPPDATA", Path::new(""))]);
    let dialog = DiskCleanupDialog::new();
    let local = home.path().join("AppData").join("Local");
    assert_eq!(
        row(&dialog, "JetBrains caches").path,
        local.join("JetBrains")
    );
    assert_eq!(
        row(&dialog, "pip cache").path,
        local.join("pip").join("Cache")
    );
}
