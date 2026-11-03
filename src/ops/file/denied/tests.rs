//! Purpose: Refusals distinguish vanished parents, ownership and file attributes.

use super::*;

#[test]
fn unrelated_errors_are_not_presented_as_permission_denials() {
    let root = tempfile::TempDir::new().unwrap();
    assert_eq!(
        classify(root.path(), &io::Error::from(io::ErrorKind::NotFound)),
        None
    );
}

#[cfg(unix)]
#[test]
fn a_parent_that_disappeared_is_named_as_the_locked_folder() {
    let root = tempfile::TempDir::new().unwrap();
    let parent = root.path().join("removed");
    let path = parent.join("file");
    assert_eq!(
        classify(&path, &io::ErrorKind::PermissionDenied.into()),
        Some(Denial::FolderLocked(parent))
    );
}

#[cfg(unix)]
#[test]
fn another_owners_metadata_is_distinguished_from_account_rights() {
    let root = tempfile::TempDir::new().unwrap();
    let path = root.path().join("file");
    fs::write(&path, b"keep").unwrap();
    let metadata = if crate::platform::effective_uid() == 0 {
        std::os::unix::fs::chown(&path, Some(65534), None).unwrap();
        fs::metadata(&path).unwrap()
    } else {
        fs::metadata("/").unwrap()
    };
    assert_eq!(refused(&path, Some(&metadata)), Denial::OtherOwner);
    assert_eq!(refused(&path, None), Denial::Rights);
    assert_eq!(fs::read(&path).unwrap(), b"keep");
}

#[cfg(windows)]
#[test]
fn windows_distinguishes_sharing_violations_and_readonly_files() {
    let root = tempfile::TempDir::new().unwrap();
    let path = root.path().join("file");
    fs::write(&path, b"keep").unwrap();
    for code in [SHARING_VIOLATION, LOCK_VIOLATION] {
        assert_eq!(
            classify(&path, &io::Error::from_raw_os_error(code)),
            Some(Denial::InUse)
        );
    }
    let mut permissions = fs::metadata(&path).unwrap().permissions();
    permissions.set_readonly(true);
    fs::set_permissions(&path, permissions).unwrap();
    assert_eq!(
        classify(&path, &io::ErrorKind::PermissionDenied.into()),
        Some(Denial::ReadOnly)
    );
    assert_eq!(refused_for_elevation(true), Denial::NotEvenAdministrator);
    assert_eq!(refused_for_elevation(false), Denial::NeedsAdministrator);
    let plain = root.path().join("plain");
    fs::write(&plain, b"x").unwrap();
    assert!(matches!(
        classify(&plain, &io::ErrorKind::PermissionDenied.into()),
        Some(Denial::NeedsAdministrator | Denial::NotEvenAdministrator)
    ));
}
