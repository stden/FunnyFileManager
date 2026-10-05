//! Purpose: The staging folder is checked for links before anything in it moves.

use std::fs;

use tempfile::TempDir;

use super::plain_tree;

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
