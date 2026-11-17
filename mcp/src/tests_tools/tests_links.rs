//! Purpose: Deleting, copying and moving a link acts on the link, never on the
//! data it points at.

use super::*;
use crate::tools::batch::BatchParams;
use crate::tools::transfer::CopyMoveParams;

// ═══════════════════════════════════════════════════════════════
// batch_delete — a link is not the tree it points at
// ═══════════════════════════════════════════════════════════════

#[cfg(unix)]
#[test]
fn batch_delete_of_a_link_to_a_directory_keeps_the_target() {
    let dir = TempDir::new().expect("temp dir");
    let (server, root) = server_at(&dir);
    write_file_at(&root.join("data").join("keep.txt"), "keep");
    std::os::unix::fs::symlink(root.join("data"), root.join("link")).expect("symlink");

    let result = server
        .batch_delete(Parameters(BatchParams {
            paths: vec!["link".to_owned()],
            destination: None,
            recursive: true,
        }))
        .expect("batch_delete returns per-path failures, not an early error");
    let payload = tool_json(&result);

    assert_eq!(payload["total_deleted"], 1);
    assert_eq!(payload["total_errors"], 0);
    assert_gone(&root.join("link"));
    assert_eq!(
        fs::read_to_string(root.join("data").join("keep.txt")).expect("the target tree survives"),
        "keep",
        "deleting a link must not delete what it points at"
    );
}

#[cfg(unix)]
#[test]
fn batch_delete_of_a_link_to_a_file_keeps_the_file() {
    let dir = TempDir::new().expect("temp dir");
    let (server, root) = server_at(&dir);
    write_file_at(&root.join("real.txt"), "payload");
    std::os::unix::fs::symlink(root.join("real.txt"), root.join("alias.txt")).expect("symlink");

    let result = server
        .batch_delete(Parameters(BatchParams {
            paths: vec!["alias.txt".to_owned()],
            destination: None,
            recursive: false,
        }))
        .expect("batch_delete returns per-path failures, not an early error");

    assert_eq!(tool_json(&result)["total_deleted"], 1);
    assert_gone(&root.join("alias.txt"));
    assert_eq!(
        fs::read_to_string(root.join("real.txt")).expect("the target survives"),
        "payload"
    );
}

// ═══════════════════════════════════════════════════════════════
// copy / move — link-ness is preserved, not dereferenced
// ═══════════════════════════════════════════════════════════════

#[cfg(unix)]
#[test]
fn batch_move_of_a_link_moves_the_link_not_its_target() {
    let dir = TempDir::new().expect("temp dir");
    let (server, root) = server_at(&dir);
    write_file_at(&root.join("real.txt"), "payload");
    std::os::unix::fs::symlink(root.join("real.txt"), root.join("alias.txt")).expect("symlink");

    let result = server
        .batch_move(Parameters(BatchParams {
            paths: vec!["alias.txt".to_owned()],
            destination: Some("moved".to_owned()),
            recursive: true,
        }))
        .expect("a batch reports per-path failures instead of aborting");
    let payload = tool_json(&result);

    assert_eq!(
        payload["moved"][0], "alias.txt",
        "the link is what was asked for, so the link is what moves"
    );
    assert_eq!(
        fs::read_to_string(root.join("real.txt")).expect("the target stays where it was"),
        "payload"
    );
    assert_gone(&root.join("alias.txt"));
    assert_is_symlink(&root.join("moved").join("alias.txt"));
}

#[cfg(unix)]
#[test]
fn batch_copy_of_a_link_reproduces_the_link() {
    let dir = TempDir::new().expect("temp dir");
    let (server, root) = server_at(&dir);
    write_file_at(&root.join("real.txt"), "payload");
    std::os::unix::fs::symlink(root.join("real.txt"), root.join("alias.txt")).expect("symlink");

    let result = server
        .batch_copy(Parameters(BatchParams {
            paths: vec!["alias.txt".to_owned()],
            destination: Some("archive".to_owned()),
            recursive: true,
        }))
        .expect("a batch reports per-path failures instead of aborting");

    assert_eq!(tool_json(&result)["copied"][0], "alias.txt");
    assert_is_symlink(&root.join("alias.txt"));
    assert_is_symlink(&root.join("archive").join("alias.txt"));
}

#[cfg(unix)]
#[test]
fn move_rename_of_a_link_moves_the_link_not_its_target() {
    let dir = TempDir::new().expect("temp dir");
    let (server, root) = server_at(&dir);
    write_file_at(&root.join("real.txt"), "payload");
    std::os::unix::fs::symlink(root.join("real.txt"), root.join("alias.txt")).expect("symlink");

    server
        .move_rename(Parameters(CopyMoveParams {
            source: "alias.txt".to_owned(),
            destination: "renamed.txt".to_owned(),
        }))
        .expect("moving a link is a rename of the link");

    assert_eq!(
        fs::read_to_string(root.join("real.txt")).expect("the target stays where it was"),
        "payload"
    );
    assert_gone(&root.join("alias.txt"));
    assert_is_symlink(&root.join("renamed.txt"));
}

#[cfg(unix)]
#[test]
fn copy_of_a_link_reproduces_the_link() {
    let dir = TempDir::new().expect("temp dir");
    let (server, root) = server_at(&dir);
    write_file_at(&root.join("real.txt"), "payload");
    std::os::unix::fs::symlink(root.join("real.txt"), root.join("alias.txt")).expect("symlink");

    server
        .copy(Parameters(CopyMoveParams {
            source: "alias.txt".to_owned(),
            destination: "copy.txt".to_owned(),
        }))
        .expect("copying a link recreates the link");

    assert_is_symlink(&root.join("alias.txt"));
    assert_is_symlink(&root.join("copy.txt"));
}

#[cfg(unix)]
#[test]
fn list_and_sort_directory_classify_directory_symlinks_as_dir() {
    let dir = TempDir::new().expect("temp dir");
    let (server, root) = server_at(&dir);
    fs::create_dir(root.join("real_dir")).expect("mkdir");
    std::os::unix::fs::symlink(root.join("real_dir"), root.join("dir_link")).expect("symlink");
    write_file_at(&root.join("aaa.txt"), "x");

    let listed = tool_json(
        &server
            .list_directory(Parameters(tools::PathParam {
                path: ".".to_owned(),
            }))
            .expect("list_directory"),
    );
    assert_eq!(listed["entries"][0]["name"], "dir_link");
    assert_eq!(listed["entries"][0]["type"], "dir");

    let sorted = tool_json(
        &server
            .sort_directory(Parameters(SortDirectoryParams {
                path: ".".to_owned(),
                sort: "name".to_owned(),
            }))
            .expect("sort_directory"),
    );
    assert_eq!(sorted["entries"][0]["name"], "dir_link");
    assert_eq!(sorted["entries"][0]["type"], "dir");
}
