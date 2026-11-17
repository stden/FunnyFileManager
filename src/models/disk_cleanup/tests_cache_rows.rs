//! Unit tests: the cache folders the cleanup rows read on each platform.

use tempfile::TempDir;

use super::tests::row;
use super::DiskCleanupDialog;
use crate::test_support::EnvVarsGuard;

#[cfg(not(any(windows, target_os = "macos")))]
#[test]
fn unix_cache_rows_live_under_the_xdg_cache_dir() {
    let home = TempDir::new().expect("tempdir");
    let caches = TempDir::new().expect("tempdir");
    let _env = EnvVarsGuard::set(&[("HOME", home.path()), ("XDG_CACHE_HOME", caches.path())]);
    let dialog = DiskCleanupDialog::new();
    for (label, folder) in [
        ("JetBrains caches", "JetBrains"),
        ("Playwright browsers", "ms-playwright"),
        ("pip cache", "pip"),
        ("uv cache", "uv"),
        ("Hugging Face cache", "huggingface"),
        ("Puppeteer browsers", "puppeteer"),
    ] {
        assert_eq!(
            row(&dialog, label).path,
            caches.path().join(folder),
            "{label}"
        );
    }
}

#[cfg(target_os = "macos")]
#[test]
fn macos_cache_rows_live_under_home_library() {
    let home = TempDir::new().expect("tempdir");
    let _env = EnvVarsGuard::set(&[("HOME", home.path())]);
    let dialog = DiskCleanupDialog::new();
    let caches = home.path().join("Library").join("Caches");
    for (label, folder) in [
        ("JetBrains caches", "JetBrains"),
        ("Playwright browsers", "ms-playwright"),
        ("pip cache", "pip"),
        ("uv cache", "uv"),
        ("Hugging Face cache", "huggingface"),
        ("Puppeteer browsers", "puppeteer"),
    ] {
        assert_eq!(row(&dialog, label).path, caches.join(folder), "{label}");
    }
}
