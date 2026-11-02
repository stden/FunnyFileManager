//! Purpose: Pins that `copy_entry` claims every destination name exclusively
//! — a dangling link is a taken name, never a hole to write through — and
//! that a copy which fails leaves nothing under the final name.

use crate::fsops::{copy_entry, Purpose};
use std::fs;
use std::io;
use tempfile::TempDir;

/// `fs::copy` follows a link at the destination and creates the file the
/// link names — which, for a link planted inside the root, can be anywhere.
#[cfg(unix)]
#[test]
fn a_dangling_link_at_the_destination_is_a_taken_name() {
    let dir = TempDir::new().expect("temp dir");
    let source = dir.path().join("mine.txt");
    fs::write(&source, "mine").expect("write");
    let elsewhere = dir.path().join("elsewhere.txt");
    let link = dir.path().join("hole.txt");
    std::os::unix::fs::symlink(&elsewhere, &link).expect("symlink");

    let error = copy_entry(&source, &link, Purpose::Copy).expect_err("the name is taken");

    assert_eq!(error.kind(), io::ErrorKind::AlreadyExists, "{error}");
    assert!(!elsewhere.exists(), "nothing was written through the link");
    assert!(
        fs::symlink_metadata(&link).unwrap().is_symlink(),
        "the link stays"
    );
}

#[cfg(unix)]
#[test]
fn a_special_file_is_refused_and_the_partial_copy_removed() {
    let dir = TempDir::new().expect("temp dir");
    let source = dir.path().join("tree");
    fs::create_dir(&source).expect("mkdir");
    fs::write(source.join("a.txt"), "a").expect("write");
    let socket = std::os::unix::net::UnixListener::bind(source.join("sock"));
    socket.expect("bind a socket in the source");
    let dest = dir.path().join("copy");

    copy_entry(&source, &dest, Purpose::Copy).expect_err("a socket cannot be copied");

    assert!(
        fs::symlink_metadata(&dest).is_err(),
        "the partial copy is removed"
    );
    assert_eq!(fs::read_to_string(source.join("a.txt")).unwrap(), "a");
}

/// A move keeps the folder modes a rename would have kept; a copy makes its
/// folders like any new folder.
#[cfg(unix)]
#[test]
fn a_moved_folder_keeps_its_mode_and_a_copied_one_is_fresh() {
    use std::os::unix::fs::PermissionsExt;

    let dir = TempDir::new().expect("temp dir");
    let source = dir.path().join("tree");
    fs::create_dir(&source).expect("mkdir");
    fs::write(source.join("a.txt"), "a").expect("write");
    fs::set_permissions(&source, fs::Permissions::from_mode(0o710)).expect("chmod");

    copy_entry(&source, &dir.path().join("moved"), Purpose::Move).expect("the copy lands");
    copy_entry(&source, &dir.path().join("copied"), Purpose::Copy).expect("the copy lands");

    assert_eq!(super::mode_of(&dir.path().join("moved")), 0o710);
    assert_ne!(super::mode_of(&dir.path().join("copied")), 0o710);
    assert_eq!(
        fs::read_to_string(dir.path().join("moved/a.txt")).unwrap(),
        "a"
    );
}

#[test]
fn a_copy_into_an_existing_folder_is_refused_without_merging() {
    let dir = TempDir::new().expect("temp dir");
    let source = dir.path().join("tree");
    fs::create_dir(&source).expect("mkdir");
    fs::write(source.join("a.txt"), "a").expect("write");
    let dest = dir.path().join("there");
    fs::create_dir(&dest).expect("mkdir");
    fs::write(dest.join("theirs.txt"), "theirs").expect("write");

    let error = copy_entry(&source, &dest, Purpose::Copy).expect_err("the name is taken");

    assert_eq!(error.kind(), io::ErrorKind::AlreadyExists, "{error}");
    assert!(!dest.join("a.txt").exists(), "nothing is merged");
    assert_eq!(
        fs::read_to_string(dest.join("theirs.txt")).unwrap(),
        "theirs"
    );
}
