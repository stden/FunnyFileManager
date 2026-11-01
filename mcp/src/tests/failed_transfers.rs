//! Purpose: A batch or a move that cannot finish reports the failure and keeps
//! the source.

use super::*;
use crate::tools::batch::BatchParams;
use crate::tools::transfer::CopyMoveParams;

// ═══════════════════════════════════════════════════════════════
// Batch failure handling
// ═══════════════════════════════════════════════════════════════

#[test]
fn batch_move_that_cannot_transfer_keeps_the_source() {
    let dir = TempDir::new().expect("temp dir");
    let (server, root) = server_at(&dir);
    write_file_at(&root.join("doc.txt"), "content");
    // The destination name is already a populated directory: `rename` refuses
    // it and the copy fallback cannot overwrite a directory either, so both
    // arms of the move fail and the source must be left untouched.
    fs::create_dir_all(root.join("out").join("doc.txt").join("blocker")).expect("mkdir");

    let result = server
        .batch_move(Parameters(BatchParams {
            paths: vec!["doc.txt".to_owned()],
            destination: Some("out".to_owned()),
            recursive: true,
        }))
        .expect("a batch reports per-path failures instead of aborting");
    let payload = tool_json(&result);

    assert_eq!(payload["total_moved"], 0);
    assert_eq!(payload["total_errors"], 1);
    assert_eq!(
        fs::read_to_string(root.join("doc.txt")).expect("the source is still readable"),
        "content",
        "a move that never landed must not consume the source"
    );
}

#[test]
fn batch_delete_without_recursion_refuses_a_populated_directory() {
    let dir = TempDir::new().expect("temp dir");
    let (server, root) = server_at(&dir);
    write_file_at(&root.join("full").join("inner.txt"), "inner");
    fs::create_dir(root.join("empty")).expect("mkdir");

    let result = server
        .batch_delete(Parameters(BatchParams {
            paths: vec!["full".to_owned(), "empty".to_owned()],
            destination: None,
            recursive: false,
        }))
        .expect("batch_delete returns per-path failures, not an early error");
    let payload = tool_json(&result);

    assert_eq!(payload["total_deleted"], 1);
    assert_eq!(payload["total_errors"], 1);
    assert!(
        root.join("full").join("inner.txt").exists(),
        "recursive=false must not wipe a populated tree"
    );
    assert!(!root.join("empty").exists());
}

// ═══════════════════════════════════════════════════════════════
// move_rename
// ═══════════════════════════════════════════════════════════════

#[test]
fn move_rename_that_cannot_copy_keeps_the_source() {
    let dir = TempDir::new().expect("temp dir");
    let (server, root) = server_at(&dir);
    write_file_at(&root.join("doc.txt"), "content");
    write_file_at(&root.join("blocked").join("child.txt"), "child");

    let error = server
        .move_rename(Parameters(CopyMoveParams {
            source: "doc.txt".to_owned(),
            destination: "blocked".to_owned(),
        }))
        .expect_err("a file cannot replace a populated directory");

    assert_ne!(error.message.len(), 0, "the failure has to be described");
    assert_eq!(
        fs::read_to_string(root.join("doc.txt")).expect("the source is still readable"),
        "content",
        "the copy fallback failed, so the source must not have been removed"
    );
    assert!(root.join("blocked").join("child.txt").exists());
}
