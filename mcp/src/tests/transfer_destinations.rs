//! Purpose: A transfer never replaces what is at its destination — an
//! existing entry, the source itself, or a place inside the source — so it
//! can never destroy data it did not move.

use super::*;
use crate::tools::batch::BatchParams;
use crate::tools::transfer::CopyMoveParams;

fn params(source: &str, destination: &str) -> Parameters<CopyMoveParams> {
    Parameters(CopyMoveParams {
        source: source.to_owned(),
        destination: destination.to_owned(),
    })
}

#[test]
fn copy_refuses_an_existing_destination_and_keeps_it() {
    let dir = TempDir::new().expect("temp dir");
    let (server, root) = server_at(&dir);
    write_file_at(&root.join("mine.txt"), "mine");
    write_file_at(&root.join("theirs.txt"), "theirs");

    let error = server
        .copy(params("mine.txt", "theirs.txt"))
        .expect_err("nothing is overwritten");

    assert!(error.message.contains("exists"), "{}", error.message);
    assert_eq!(
        fs::read_to_string(root.join("theirs.txt")).unwrap(),
        "theirs"
    );
}

/// `fs::copy` onto the same file truncates it before reading a byte.
#[test]
fn copy_onto_itself_keeps_the_file() {
    let dir = TempDir::new().expect("temp dir");
    let (server, root) = server_at(&dir);
    write_file_at(&root.join("mine.txt"), "mine");

    server
        .copy(params("mine.txt", "mine.txt"))
        .expect_err("a file is not copied onto itself");

    assert_eq!(fs::read_to_string(root.join("mine.txt")).unwrap(), "mine");
}

/// The copy would descend into the copy it is writing until the path length
/// gave out.
#[test]
fn copy_of_a_folder_into_itself_is_refused() {
    let dir = TempDir::new().expect("temp dir");
    let (server, root) = server_at(&dir);
    write_file_at(&root.join("tree").join("x.txt"), "x");

    let error = server
        .copy(params("tree", "tree/sub"))
        .expect_err("a folder is not copied into itself");

    assert!(error.message.contains("into itself"), "{}", error.message);
    assert!(!root.join("tree/sub").exists(), "nothing was started");
    assert_eq!(fs::read_to_string(root.join("tree/x.txt")).unwrap(), "x");
}

#[test]
fn move_rename_refuses_an_existing_destination_and_keeps_both() {
    let dir = TempDir::new().expect("temp dir");
    let (server, root) = server_at(&dir);
    write_file_at(&root.join("mine.txt"), "mine");
    write_file_at(&root.join("theirs.txt"), "theirs");

    let error = server
        .move_rename(params("mine.txt", "theirs.txt"))
        .expect_err("a rename would replace the destination");

    assert!(error.message.contains("exists"), "{}", error.message);
    assert_eq!(fs::read_to_string(root.join("mine.txt")).unwrap(), "mine");
    assert_eq!(
        fs::read_to_string(root.join("theirs.txt")).unwrap(),
        "theirs"
    );
}

#[test]
fn batch_copy_refuses_a_taken_name_and_copies_the_rest() {
    let dir = TempDir::new().expect("temp dir");
    let (server, root) = server_at(&dir);
    write_file_at(&root.join("a.txt"), "mine");
    write_file_at(&root.join("c.txt"), "c");
    write_file_at(&root.join("out").join("a.txt"), "theirs");

    let result = server
        .batch_copy(Parameters(BatchParams {
            paths: vec!["a.txt".to_owned(), "c.txt".to_owned()],
            destination: Some("out".to_owned()),
            recursive: true,
        }))
        .expect("a batch reports per-path failures instead of aborting");
    let payload = tool_json(&result);

    assert_eq!(payload["copied"][0], "c.txt");
    assert_eq!(payload["total_errors"], 1);
    assert_eq!(
        fs::read_to_string(root.join("out/a.txt")).unwrap(),
        "theirs"
    );
}
