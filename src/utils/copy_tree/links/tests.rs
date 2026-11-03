//! Unit tests: platform symlink creation and rollback.

use super::{create_platform_link, remove_created_link};
use tempfile::TempDir;

#[test]
fn create_and_remove_a_link() {
    let dir = TempDir::new().expect("temp");
    let target = dir.path().join("target.txt");
    let link = dir.path().join("link.txt");
    std::fs::write(&target, b"content").expect("write target");

    create_platform_link(&target, &target, &link).expect("create link");
    assert!(link.exists() || std::fs::symlink_metadata(&link).is_ok());

    remove_created_link(&link);
    assert!(!std::fs::symlink_metadata(&link).is_ok_and(|_| true));
}

#[test]
fn remove_created_link_is_silent_for_missing_path() {
    let dir = TempDir::new().expect("temp");
    // Removing a non-existent path must not panic.
    remove_created_link(&dir.path().join("no_such_link"));
}

/// Windows: the TEST_LINK_FALLBACK thread-local lets tests exercise the
/// dir-link and file-link branches without `SeCreateSymbolicLinkPrivilege`.
#[cfg(all(windows, test))]
mod windows_fallback {
    use super::super::{create_windows_link, TEST_LINK_FALLBACK};

    fn with_fallback<F: FnOnce()>(as_dir: bool, f: F) {
        TEST_LINK_FALLBACK.with(|cell| cell.set(Some(as_dir)));
        f();
        TEST_LINK_FALLBACK.with(|cell| cell.set(None));
    }

    #[test]
    fn fallback_creates_a_directory_for_dir_links() {
        let dir = tempfile::TempDir::new().expect("temp");
        let dst = dir.path().join("dir_link");
        with_fallback(true, || {
            create_windows_link(std::path::Path::new("unused"), &dst, true)
                .expect("fallback dir link");
        });
        assert!(dst.is_dir());
    }

    #[test]
    fn fallback_creates_a_file_for_file_links() {
        let dir = tempfile::TempDir::new().expect("temp");
        let dst = dir.path().join("file_link");
        with_fallback(false, || {
            create_windows_link(std::path::Path::new("unused"), &dst, false)
                .expect("fallback file link");
        });
        assert!(dst.is_file());
    }
}
