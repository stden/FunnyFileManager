//! Purpose: Unit tests for ancestor canonicalization and symlink follow policy.

use super::{canonical_existing_ancestor, follows_link};
use crate::utils::LinkPolicy;
use std::fs;
#[cfg(windows)]
use std::path::Path;

#[test]
fn canonical_existing_ancestor_walks_up_missing_segments() {
    let temp = tempfile::TempDir::new().unwrap();
    let nested = temp.path().join("missing").join("leaf.txt");
    assert_eq!(
        canonical_existing_ancestor(&nested),
        Some(fs::canonicalize(temp.path()).unwrap())
    );
}

#[test]
fn follows_link_checks_policy_and_regular_or_dir_targets() {
    let temp = tempfile::TempDir::new().unwrap();
    let file = temp.path().join("f.txt");
    fs::write(&file, b"x").unwrap();
    assert!(!follows_link(LinkPolicy::Recreate, &file));
    assert!(follows_link(LinkPolicy::Follow, &file));
    assert!(follows_link(LinkPolicy::Follow, temp.path()));
    assert!(!follows_link(
        LinkPolicy::Follow,
        &temp.path().join("missing")
    ));
    #[cfg(windows)]
    assert!(!follows_link(LinkPolicy::Follow, Path::new("NUL")));
}

#[test]
fn copy_link_and_followed_walk_guard_detect_cycles_and_preserve_folder_metadata() {
    use super::{copy_dir_all_with, copy_link, CopyRules};
    use crate::utils::{FileMode, Times};

    let temp = tempfile::TempDir::new().unwrap();
    let ext = temp.path().join("ext");
    fs::create_dir(&ext).unwrap();
    fs::write(ext.join("a.txt"), b"ok").unwrap();
    let link = temp.path().join("j_ext");
    crate::test_support::make_dir_link(&ext, &link).unwrap();
    drop(copy_link(
        &link,
        &temp.path().join("j_copy"),
        CopyRules::PLAIN,
    ));

    let follow = CopyRules {
        links: LinkPolicy::Follow,
        modes: FileMode::Source,
        folder_modes: FileMode::Source,
        times: Times::Source,
    };
    let src_ok = temp.path().join("src_ok");
    fs::create_dir(&src_ok).unwrap();
    crate::test_support::make_dir_link(&ext, &src_ok.join("linked")).unwrap();
    let dst_ok = temp.path().join("dst_ok");
    copy_dir_all_with(&src_ok, &dst_ok, follow).unwrap();
    assert_eq!(
        fs::read(dst_ok.join("linked").join("a.txt")).unwrap(),
        b"ok"
    );
}

#[test]
fn followed_walk_rejects_links_into_ancestor_or_output_tree() {
    use super::{copy_dir_all_with, CopyRules};
    use crate::utils::{FileMode, Times};

    let follow = CopyRules {
        links: LinkPolicy::Follow,
        modes: FileMode::Fresh,
        folder_modes: FileMode::Fresh,
        times: Times::Fresh,
    };
    let temp = tempfile::TempDir::new().unwrap();
    let src_loop = temp.path().join("src_loop");
    fs::create_dir(&src_loop).unwrap();
    crate::test_support::make_dir_link(&src_loop, &src_loop.join("up")).unwrap();
    let loop_err = copy_dir_all_with(&src_loop, &temp.path().join("out1"), follow).unwrap_err();
    assert!(loop_err.to_string().contains("symbolic link loop"));

    let src_into = temp.path().join("src_into");
    let out2 = temp.path().join("out2");
    fs::create_dir(&src_into).unwrap();
    fs::create_dir(&out2).unwrap();
    crate::test_support::make_dir_link(&out2, &src_into.join("self_out")).unwrap();
    let into_err = copy_dir_all_with(&src_into, &out2, follow).unwrap_err();
    assert!(into_err.to_string().contains("leads into the copy itself"));
}

struct TimesFailureGuard;

impl Drop for TimesFailureGuard {
    fn drop(&mut self) {
        crate::utils::keep_times::FORCE_TIMES_FAILURE.set(false);
        #[cfg(windows)]
        super::links::TEST_LINK_FALLBACK.set(None);
    }
}

#[test]
fn copy_file_checked_preserves_destination_error_or_prefixes_source_path() {
    let temp = tempfile::TempDir::new().unwrap();
    let src = temp.path().join("src.txt");
    let taken = temp.path().join("taken.txt");
    fs::write(&src, b"a").unwrap();
    fs::write(&taken, b"b").unwrap();

    let dst_err =
        super::steps::copy_file_checked(&src, &taken, super::CopyRules::PLAIN).unwrap_err();
    assert!(crate::utils::FailedAt::of(&dst_err).is_some());

    let missing = temp.path().join("missing.txt");
    let fresh = temp.path().join("fresh.txt");
    let src_err =
        super::steps::copy_file_checked(&missing, &fresh, super::CopyRules::PLAIN).unwrap_err();
    assert!(crate::utils::FailedAt::of(&src_err).is_none());
    assert!(src_err.to_string().contains("missing.txt"));
}

#[test]
fn recreate_links_keep_times_and_classify_windows_reparse_points() {
    use super::{copy_dir_all_with, copy_link, CopyRules};
    let _reset = TimesFailureGuard;
    let temp = tempfile::TempDir::new().unwrap();
    let target_dir = temp.path().join("target_dir");
    fs::create_dir(&target_dir).unwrap();
    let file = target_dir.join("note.txt");
    fs::write(&file, b"hi").unwrap();
    let tree_src = temp.path().join("tree_src");
    fs::create_dir(&tree_src).unwrap();
    let link = tree_src.join("dir_link");
    crate::test_support::make_dir_link(&target_dir, &link).unwrap();
    #[cfg(windows)]
    {
        assert!(super::links::is_windows_dir_link(&link));
        assert!(super::links::is_windows_dir_link(&target_dir));
        assert!(!super::links::is_windows_dir_link(&file));
        let (d_try, f_try) = (temp.path().join("d_try"), temp.path().join("f_try"));
        drop(super::links::create_windows_link(&target_dir, &d_try, true));
        drop(fs::remove_dir(&d_try));
        drop(super::links::create_windows_link(&file, &f_try, false));
        drop(fs::remove_file(&f_try));
        super::links::TEST_LINK_FALLBACK.set(Some(true));
    }
    copy_link(&link, &temp.path().join("fresh_link"), CopyRules::PLAIN).unwrap();
    copy_link(&link, &temp.path().join("timed_link"), CopyRules::MOVE).unwrap();
    copy_dir_all_with(&tree_src, &temp.path().join("tree_out"), CopyRules::PLAIN).unwrap();
}

#[test]
fn recreate_links_roll_back_on_timestamp_failure() {
    use super::{copy_dir_all_with, copy_link, CopyRules};
    let _reset = TimesFailureGuard;
    let temp = tempfile::TempDir::new().unwrap();
    let target_dir = temp.path().join("target_dir");
    fs::create_dir(&target_dir).unwrap();
    let tree_src = temp.path().join("tree_src");
    fs::create_dir(&tree_src).unwrap();
    let link = tree_src.join("dir_link");
    crate::test_support::make_dir_link(&target_dir, &link).unwrap();
    #[cfg(windows)]
    super::links::TEST_LINK_FALLBACK.set(Some(true));
    crate::utils::keep_times::FORCE_TIMES_FAILURE.set(true);
    let rollback_dir = temp.path().join("rollback_dir");
    assert!(copy_link(&link, &rollback_dir, CopyRules::MOVE).is_err());
    assert!(fs::symlink_metadata(&rollback_dir).is_err());
    assert!(copy_dir_all_with(&tree_src, &temp.path().join("tree_fail"), CopyRules::MOVE).is_err());
    #[cfg(windows)]
    {
        super::links::TEST_LINK_FALLBACK.set(Some(false));
        let rollback_file = temp.path().join("rollback_file");
        assert!(copy_link(&link, &rollback_file, CopyRules::MOVE).is_err());
        assert!(fs::symlink_metadata(&rollback_file).is_err());
    }
}
