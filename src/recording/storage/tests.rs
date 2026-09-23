//! Purpose: Recording storage refuses redirected directories and repairs permissive modes.

use super::*;
use std::os::unix::fs::{symlink, PermissionsExt};
use tempfile::TempDir;

#[test]
fn a_symlink_cannot_redirect_private_keystroke_storage() {
    let root = TempDir::new().unwrap();
    let target = root.path().join("target");
    let link = root.path().join("link");
    std::fs::create_dir(&target).unwrap();
    symlink(&target, &link).unwrap();
    assert_eq!(
        ensure_private(&link).unwrap_err().kind(),
        io::ErrorKind::PermissionDenied
    );
    assert!(target.is_dir());
}

#[test]
fn an_existing_shared_mode_is_tightened_before_recording() {
    let root = TempDir::new().unwrap();
    let path = root.path().join("recordings");
    std::fs::create_dir(&path).unwrap();
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).unwrap();
    create_private_dir(&path).unwrap();
    assert_eq!(
        std::fs::metadata(path).unwrap().permissions().mode() & 0o777,
        0o700
    );
}
