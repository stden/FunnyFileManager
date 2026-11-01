//! Purpose: Pins that `remove_entry` removes files, directories and links
//! without ever following a link to its target.

use crate::fsops::remove_entry;
use std::fs;
use tempfile::TempDir;

#[cfg(unix)]
#[test]
fn deleting_a_link_to_a_directory_keeps_the_target() {
    let dir = TempDir::new().expect("temp dir");
    let tree = dir.path().join("data/tree");
    fs::create_dir_all(&tree).expect("mkdir");
    fs::write(tree.join("keep.txt"), "keep").expect("write");

    let link = dir.path().join("link_to_tree");
    std::os::unix::fs::symlink(&tree, &link).expect("symlink");

    remove_entry(&link, true).expect("removing the link must succeed");

    assert!(!link.is_symlink(), "the link itself should be gone");
    assert!(
        tree.join("keep.txt").exists(),
        "the target tree must survive"
    );
}

#[cfg(unix)]
#[test]
fn deleting_a_dangling_link_succeeds() {
    let dir = TempDir::new().expect("temp dir");
    let link = dir.path().join("dangling");
    std::os::unix::fs::symlink(dir.path().join("no_such_target"), &link).expect("symlink");

    remove_entry(&link, true).expect("a dangling link must still be removable");
    assert!(!link.is_symlink());
}

#[test]
fn removes_files_and_directories() {
    let dir = TempDir::new().expect("temp dir");
    let file = dir.path().join("f.txt");
    fs::write(&file, "x").expect("write");
    remove_entry(&file, false).expect("remove file");
    assert!(!file.exists());

    let nested = dir.path().join("d/inner");
    fs::create_dir_all(&nested).expect("mkdir");
    remove_entry(&dir.path().join("d"), false).expect_err("non-recursive remove of a full dir");
    remove_entry(&dir.path().join("d"), true).expect("recursive remove");
    assert!(!nested.exists());
}

#[test]
fn missing_path_is_reported_not_ignored() {
    let dir = TempDir::new().expect("temp dir");
    remove_entry(&dir.path().join("ghost"), true).expect_err("missing path must error");
}
