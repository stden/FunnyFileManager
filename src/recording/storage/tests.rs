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

/// The temporary directory is shared, so another user can create this user's
/// recordings directory first. It is refused, and left exactly as it is: the
/// keystrokes must not go where somebody else chose.
#[test]
fn a_directory_owned_by_another_user_is_refused_and_left_alone() {
    let root = TempDir::new().unwrap();
    let path = root.path().join("recordings");
    std::fs::create_dir(&path).unwrap();
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o777)).unwrap();
    let someone_else = crate::platform::real_uid().wrapping_add(1);

    let error = ensure_private_to(&path, someone_else).unwrap_err();

    assert_eq!(error.kind(), io::ErrorKind::PermissionDenied);
    assert!(
        error
            .to_string()
            .starts_with("not a directory of your own: "),
        "{error}"
    );
    assert_eq!(
        std::fs::metadata(&path).unwrap().permissions().mode() & 0o777,
        0o777,
        "a refused directory is not tightened"
    );
}
