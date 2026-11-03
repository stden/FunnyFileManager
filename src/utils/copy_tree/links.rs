//! Purpose: platform symlink creation and rollback helpers for directory and file copies.

use std::{fs, io, path::Path};

#[cfg(all(windows, test))]
thread_local! {
    pub(super) static TEST_LINK_FALLBACK: std::cell::Cell<Option<bool>> =
        const { std::cell::Cell::new(None) };
}

#[cfg(unix)]
pub(super) fn create_platform_link(_src: &Path, target: &Path, dst: &Path) -> io::Result<()> {
    std::os::unix::fs::symlink(target, dst)
}

#[cfg(windows)]
pub(super) fn create_platform_link(src: &Path, target: &Path, dst: &Path) -> io::Result<()> {
    let is_dir_link = is_windows_dir_link(src);
    create_windows_link(target, dst, is_dir_link)
}

#[cfg(windows)]
pub(super) fn is_windows_dir_link(src: &Path) -> bool {
    use std::os::windows::fs::FileTypeExt;
    fs::symlink_metadata(src).is_ok_and(|meta| meta.file_type().is_symlink_dir()) || src.is_dir()
}

#[cfg(windows)]
pub(super) fn create_windows_link(target: &Path, dst: &Path, is_dir_link: bool) -> io::Result<()> {
    #[cfg(test)]
    if let Some(as_dir) = TEST_LINK_FALLBACK.get() {
        return if as_dir {
            fs::create_dir(dst)
        } else {
            fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(dst)
                .map(|_| ())
        };
    }
    if is_dir_link {
        std::os::windows::fs::symlink_dir(target, dst)
    } else {
        std::os::windows::fs::symlink_file(target, dst)
    }
}

#[cfg(unix)]
pub(super) fn remove_created_link(dst: &Path) {
    drop(fs::remove_file(dst));
}

/// Best effort: the times' error is the one worth reporting. On Windows a
/// directory symlink requires `remove_dir` rather than `remove_file`.
#[cfg(not(unix))]
pub(super) fn remove_created_link(dst: &Path) {
    if fs::remove_file(dst).is_err() {
        drop(fs::remove_dir(dst));
    }
}
