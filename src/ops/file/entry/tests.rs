//! Purpose: Unit tests for Windows read-only removal and permission restoration on failure.

#[cfg(windows)]
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

#[cfg(windows)]
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
fn copy_entry_and_remove_path_handle_directory_links_without_touching_targets() {
    use super::{copy_entry, remove_path};
    use crate::utils::CopyRules;

    let temp = tempfile::TempDir::new().unwrap();
    let target = temp.path().join("target_dir");
    std::fs::create_dir(&target).unwrap();
    std::fs::write(target.join("keep.txt"), b"safe").unwrap();
    let link = temp.path().join("dir_link");
    crate::test_support::make_dir_link(&target, &link).unwrap();

    drop(copy_entry(
        &link,
        &temp.path().join("link_copy"),
        CopyRules::PLAIN,
    ));
    remove_path(&link).unwrap();
    assert!(std::fs::symlink_metadata(&link).is_err());
    assert_eq!(std::fs::read(target.join("keep.txt")).unwrap(), b"safe");
}

#[test]
fn copy_entry_removes_partially_claimed_directory_when_followed_link_loops() {
    use super::copy_entry;
    use crate::utils::{CopyRules, LinkPolicy};

    let temp = tempfile::TempDir::new().unwrap();
    let src = temp.path().join("src_dir");
    std::fs::create_dir(&src).unwrap();
    crate::test_support::make_dir_link(&src, &src.join("loop")).unwrap();
    let dst = temp.path().join("dst_dir");
    let rules = CopyRules {
        links: LinkPolicy::Follow,
        ..CopyRules::PLAIN
    };
    assert!(copy_entry(&src, &dst, rules).is_err());
    assert!(!dst.exists());
}

#[cfg(windows)]
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

#[cfg(windows)]
#[test]
fn move_replace_over_windows_dir_symlink_stages_across_denial() {
    use crate::copy::{CopyPrompt, Purpose};
    use std::fs;

    let _guard = super::super::RENAME_TEST_MUTEX
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let temp = tempfile::TempDir::new().unwrap();
    let src = temp.path().join("src.txt");
    fs::write(&src, b"moved").unwrap();
    let target = temp.path().join("target_dir");
    fs::create_dir(&target).unwrap();
    let dest = temp.path().join("dest_link");
    crate::test_support::make_dir_link(&target, &dest).unwrap();
    let prompt = CopyPrompt::for_purpose(Purpose::Move, "src.txt", &dest);
    super::super::FORCE_RENAME_DENIAL.set(true);
    let res = super::super::replace::entry(&src, &dest, &prompt);
    super::super::FORCE_RENAME_DENIAL.set(false);
    res.unwrap();
    assert_eq!(fs::read(&dest).unwrap(), b"moved");
    assert!(!src.exists());
    assert!(target.is_dir());
}
