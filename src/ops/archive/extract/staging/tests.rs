//! Purpose: The staging folder is checked for links and read-only directories before anything in it moves.

use std::fs;
use std::os::unix::fs::PermissionsExt;

use tempfile::TempDir;

use super::{plain_tree, settle};

/// What unpacked must be plain files and folders before any of it moves
/// into the destination; a link that got past the listing stops it there.
#[test]
fn a_link_unpacked_into_the_staging_folder_is_refused() {
    let temp = TempDir::new().expect("temp");
    fs::create_dir_all(temp.path().join("docs")).expect("docs");
    fs::write(temp.path().join("docs/a.txt"), "a").expect("file");
    std::os::unix::fs::symlink("/etc", temp.path().join("docs/etc")).expect("link");

    let error = plain_tree(temp.path()).expect_err("a link");

    assert!(error.to_string().contains("docs/etc"), "{error}");
}

#[test]
fn settle_merges_readonly_staged_directory_into_existing_destination_folder() {
    let staging = TempDir::new().expect("staging");
    let dest = TempDir::new().expect("dest");
    let staged_sub = staging.path().join("sub");
    fs::create_dir_all(&staged_sub).expect("staged sub");
    fs::write(staged_sub.join("file.txt"), "hello").expect("file");
    fs::set_permissions(&staged_sub, fs::Permissions::from_mode(0o555)).expect("chmod 0555");

    fs::create_dir_all(dest.path().join("sub")).expect("dest sub");
    settle(staging.path(), dest.path()).expect("settle should succeed on 0555 staged folder");
    assert_eq!(
        fs::read_to_string(dest.path().join("sub/file.txt")).expect("read"),
        "hello"
    );
}
