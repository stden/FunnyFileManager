//! Purpose: Tool calls that name a path outside the server root are refused
//! and change nothing.

use super::*;
use crate::tools::batch::BatchParams;
use crate::tools::writing::WriteFileParams;
use crate::tools::PathParam;

// ═══════════════════════════════════════════════════════════════
// Sandbox refusals
// ═══════════════════════════════════════════════════════════════

#[test]
fn list_directory_rejects_a_path_outside_the_root() {
    let dir = TempDir::new().expect("temp dir");
    let (server, root) = server_at(&dir);

    let error = server
        .list_directory(Parameters(PathParam {
            path: "/etc".to_owned(),
        }))
        .expect_err("/etc is not inside the sandbox");

    assert!(
        error.message.contains("outside the allowed root"),
        "got: {}",
        error.message
    );
    assert!(
        error.message.contains(&root.display().to_string()),
        "the refusal must name the root an operator can change, got: {}",
        error.message
    );
}

#[test]
fn write_file_outside_the_root_creates_nothing() {
    let dir = TempDir::new().expect("temp dir");
    let outside = TempDir::new().expect("temp dir");
    let (server, root) = server_at(&dir);
    let target = outside.path().join("pwned.txt");

    let error = server
        .write_file(Parameters(WriteFileParams {
            path: target.display().to_string(),
            content: "pwned".to_owned(),
        }))
        .expect_err("writing outside the sandbox must be refused");

    assert!(
        error.message.contains("outside the allowed root"),
        "got: {}",
        error.message
    );
    assert!(
        error.message.contains(&root.display().to_string()),
        "got: {}",
        error.message
    );
    assert!(
        !target.exists(),
        "the refusal has to happen before anything is written"
    );
}

#[test]
fn batch_delete_refuses_a_path_outside_the_root_and_leaves_it_alone() {
    let dir = TempDir::new().expect("temp dir");
    let outside = TempDir::new().expect("temp dir");
    let (server, root) = server_at(&dir);
    let victim = outside.path().join("keep.txt");
    write_file_at(&victim, "keep");
    write_file_at(&root.join("inside.txt"), "gone");

    let result = server.batch_delete(Parameters(BatchParams {
        paths: vec![victim.display().to_string(), "inside.txt".to_owned()],
        destination: None,
        recursive: true,
    })).expect("batch_delete returns per-path failures, not an early error");
    let payload = tool_json(&result);

    assert_eq!(payload["total_deleted"], 1);
    assert_eq!(payload["total_errors"], 1);
    assert!(
        payload["errors"][0]["error"]
            .as_str()
            .expect("the failure carries a message")
            .contains("outside the allowed root"),
        "got: {}",
        payload["errors"][0]
    );
    assert!(
        victim.exists(),
        "a path outside the root must survive the batch"
    );
    assert!(!root.join("inside.txt").exists());
}

#[test]
fn batch_copy_refuses_a_source_outside_the_root() {
    let dir = TempDir::new().expect("temp dir");
    let outside = TempDir::new().expect("temp dir");
    let (server, root) = server_at(&dir);
    let secret = outside.path().join("secret.txt");
    write_file_at(&secret, "secret");

    let result = server
        .batch_copy(Parameters(BatchParams {
            paths: vec![secret.display().to_string()],
            destination: Some("stolen".to_owned()),
            recursive: true,
        }))
        .expect("a batch reports per-path failures instead of aborting");
    let payload = tool_json(&result);

    assert_eq!(payload["total_copied"], 0);
    assert_eq!(payload["total_errors"], 1);
    assert!(
        payload["errors"][0]["error"]
            .as_str()
            .expect("the failure carries a message")
            .contains("outside the allowed root"),
        "got: {}",
        payload["errors"][0]
    );
    assert!(
        !root.join("stolen").join("secret.txt").exists(),
        "nothing outside the root may be copied in"
    );
}

#[test]
fn batch_move_refuses_a_source_outside_the_root() {
    let dir = TempDir::new().expect("temp dir");
    let outside = TempDir::new().expect("temp dir");
    let (server, root) = server_at(&dir);
    let secret = outside.path().join("secret.txt");
    write_file_at(&secret, "secret");

    let result = server
        .batch_move(Parameters(BatchParams {
            paths: vec![secret.display().to_string()],
            destination: Some("stolen".to_owned()),
            recursive: true,
        }))
        .expect("a batch reports per-path failures instead of aborting");
    let payload = tool_json(&result);

    assert_eq!(payload["total_moved"], 0);
    assert_eq!(payload["total_errors"], 1);
    assert!(
        secret.exists(),
        "a refused move must not remove the source either"
    );
    assert!(!root.join("stolen").join("secret.txt").exists());
}
