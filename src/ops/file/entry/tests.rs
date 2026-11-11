//! Purpose: Unit tests for copying one entry: directory links and link loops,
//! and a move that replaces a Windows directory link.

#[test]
fn copy_entry_and_remove_path_handle_directory_links_without_touching_targets() {
    use super::super::removal::remove_path;
    use super::copy_entry;
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
