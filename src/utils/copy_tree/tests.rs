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
