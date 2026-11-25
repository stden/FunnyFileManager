//! Purpose: Unit tests on Windows for read-only removal, directory links, and
//! the permission restored when a removal still fails.

#[test]
fn remove_path_clears_windows_readonly_attribute_on_file_and_empty_dir() {
    use super::{remove_path, remove_read_only};
    use std::fs;
    let temp = tempfile::TempDir::new().unwrap();
    let file = temp.path().join("ro.txt");
    fs::write(&file, b"locked").unwrap();
    let mut perms = fs::metadata(&file).unwrap().permissions();
    perms.set_readonly(true);
    fs::set_permissions(&file, perms.clone()).unwrap();
    remove_path(&file).unwrap();
    assert!(!file.exists());

    let dir = temp.path().join("ro_dir");
    fs::create_dir(&dir).unwrap();
    fs::set_permissions(&dir, perms).unwrap();
    remove_read_only(&dir, true).unwrap();
    assert!(!dir.exists());
}

#[test]
fn remove_read_only_rejects_writable_and_restores_readonly_on_non_empty_dir() {
    use super::remove_read_only;
    use std::{fs, io};
    let temp = tempfile::TempDir::new().unwrap();
    let dir = temp.path().join("non_empty");
    fs::create_dir(&dir).unwrap();
    fs::write(dir.join("child.txt"), b"x").unwrap();
    assert_eq!(
        remove_read_only(&dir, true).unwrap_err().kind(),
        io::ErrorKind::PermissionDenied
    );
    let mut perms = fs::metadata(&dir).unwrap().permissions();
    perms.set_readonly(true);
    fs::set_permissions(&dir, perms).unwrap();
    assert!(remove_read_only(&dir, true).is_err());
    assert!(fs::metadata(&dir).unwrap().permissions().readonly());
}

#[test]
fn remove_link_or_file_handles_readonly_and_missing_symlink_dir_fallbacks() {
    use super::remove_link_or_file;
    use std::fs;

    let temp = tempfile::TempDir::new().unwrap();
    let target = temp.path().join("target");
    fs::create_dir(&target).unwrap();
    let link = temp.path().join("junc");
    crate::test_support::make_dir_link(&target, &link).unwrap();
    let link_ft = fs::symlink_metadata(&link).unwrap().file_type();
    fs::remove_dir(&link).unwrap();
    assert!(remove_link_or_file(&link, link_ft).is_err());

    let ro_dir = temp.path().join("ro_empty");
    fs::create_dir(&ro_dir).unwrap();
    let mut ro = fs::metadata(&ro_dir).unwrap().permissions();
    ro.set_readonly(true);
    fs::set_permissions(&ro_dir, ro.clone()).unwrap();
    remove_link_or_file(&ro_dir, link_ft).unwrap();
    assert!(!ro_dir.exists());

    let reg = temp.path().join("reg.txt");
    fs::write(&reg, b"x").unwrap();
    let reg_ft = fs::symlink_metadata(&reg).unwrap().file_type();
    let busy_dir = temp.path().join("busy_ro");
    fs::create_dir(&busy_dir).unwrap();
    fs::write(busy_dir.join("in.txt"), b"x").unwrap();
    fs::set_permissions(&busy_dir, ro).unwrap();
    assert!(remove_link_or_file(&busy_dir, reg_ft).is_err());
}
