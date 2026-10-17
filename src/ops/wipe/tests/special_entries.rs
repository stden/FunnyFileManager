//! Purpose: Links, FIFOs, sockets and hard-linked files lose only their name.

use super::*;

#[cfg(unix)]
#[test]
fn a_symlink_is_unlinked_and_its_target_is_untouched() {
    let dir = TempDir::new().expect("temp dir");
    let target = dir.path().join("target.txt");
    fs::write(&target, b"keep me").expect("write");
    let link = dir.path().join("link");
    std::os::unix::fs::symlink(&target, &link).expect("symlink");
    let witness = Witness::install();

    wipe_path(&link).expect("wipe");

    assert!(witness.at(Stage::Overwritten).is_empty(), "never followed");
    assert!(fs::symlink_metadata(&link).is_err(), "the link is gone");
    assert_eq!(fs::read(&target).expect("read"), b"keep me");
}

/// How long a wipe of a special file may take before the test calls it hung.
#[cfg(unix)]
const HANG: std::time::Duration = std::time::Duration::from_secs(5);

/// A pipe has no bytes to cover, and opening one to write blocks until a
/// reader comes — on the thread that draws the UI. Only its name goes.
#[cfg(unix)]
#[test]
fn a_fifo_is_unlinked_without_being_opened() {
    let dir = TempDir::new().expect("temp dir");
    let fifo = dir.path().join("pipe");
    crate::test_support::make_fifo(&fifo).expect("mkfifo");

    let target = fifo.clone();
    let result = crate::test_support::run_with_timeout(HANG, move || wipe_path(&target));

    let result = result.expect("the wipe hung opening the pipe");
    result.expect("wipe");
    assert!(fs::symlink_metadata(&fifo).is_err(), "the pipe is gone");
}

/// The same pipe inside a folder: the folder and its other files still go.
#[cfg(unix)]
#[test]
fn a_directory_holding_a_fifo_is_wiped_without_hanging() {
    let dir = TempDir::new().expect("temp dir");
    let root = dir.path().join("special");
    fs::create_dir(&root).expect("mkdir");
    crate::test_support::make_fifo(&root.join("pipe")).expect("mkfifo");
    fs::write(root.join("plain.txt"), b"data").expect("write");

    let target = root.clone();
    let result = crate::test_support::run_with_timeout(HANG, move || wipe_path(&target));

    result.expect("the wipe hung").expect("wipe");
    assert!(!root.exists());
}

/// A socket cannot be opened as a file at all; wiping one used to fail
/// instead of removing it.
#[cfg(unix)]
#[test]
fn a_socket_is_unlinked_without_being_opened() {
    let dir = TempDir::new().expect("temp dir");
    let socket = dir.path().join("socket");
    let listener = std::os::unix::net::UnixListener::bind(&socket).expect("bind");

    wipe_path(&socket).expect("wipe");

    assert!(fs::symlink_metadata(&socket).is_err(), "the socket is gone");
    drop(listener);
}

/// A hard link shares its bytes with every other name of the file: zeroing
/// them through one name empties them all. Only this name goes.
#[cfg(unix)]
#[test]
fn a_hard_linked_file_is_only_unlinked_and_its_other_name_keeps_the_data() {
    let dir = TempDir::new().expect("temp dir");
    let wiped = dir.path().join("hl-a");
    let other = dir.path().join("hl-b");
    fs::write(&wiped, b"important data").expect("write");
    fs::hard_link(&wiped, &other).expect("hard link");
    let witness = Witness::install();

    wipe_path(&wiped).expect("wipe");

    assert!(!wiped.exists(), "this name is gone");
    assert_eq!(fs::read(&other).expect("read"), b"important data");
    assert!(witness.at(Stage::Overwritten).is_empty(), "never zeroed");
}

/// A folder holding a hard link to a file outside it: the folder goes, the
/// outside file keeps its bytes.
#[cfg(unix)]
#[test]
fn a_directory_wipe_leaves_the_data_of_files_linked_from_outside() {
    let dir = TempDir::new().expect("temp dir");
    let outside = dir.path().join("medium.txt");
    fs::write(&outside, b"forty lines").expect("write");
    let root = dir.path().join("links");
    fs::create_dir(&root).expect("mkdir");
    fs::hard_link(&outside, root.join("hardlink-medium.txt")).expect("hard link");

    wipe_path(&root).expect("wipe");

    assert!(!root.exists());
    assert_eq!(fs::read(&outside).expect("read"), b"forty lines");
}

/// The status bar says which files were only unlinked, so a user who meant
/// to destroy the data knows it is still there under another name.
#[cfg(unix)]
#[test]
fn run_says_when_a_hard_linked_file_was_only_unlinked() {
    let dir = TempDir::new().expect("temp dir");
    fs::write(dir.path().join("hl-a"), b"important data").expect("write");
    fs::hard_link(dir.path().join("hl-a"), dir.path().join("hl-b")).expect("hard link");
    let mut app = app_in(dir.path());
    app.set_overlay(Overlay::WipeConfirm(vec![dir.path().join("hl-a")]));

    run(&mut app);

    assert_eq!(
        status_text(&app),
        "Wiped 1 item(s); 1 hard-linked file(s) only unlinked, their data stays under the other names"
    );
}

#[test]
fn a_missing_entry_is_an_error() {
    let dir = TempDir::new().expect("temp dir");
    let error = wipe_path(&dir.path().join("absent")).expect_err("must fail");
    assert_eq!(error.kind(), std::io::ErrorKind::NotFound);
}
