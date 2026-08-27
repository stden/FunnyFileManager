//! In-process tests for the rules that must hold at *every* call site: the
//! registered tool list, and the promise that a symlink is never silently
//! stood in for the data it points at.
//!
//! These live beside `tests.rs` rather than inside it because they are
//! cross-cutting: each one has a twin somewhere else in `main.rs`, and keeping
//! them together is what makes a missing twin obvious.

use super::*;
use tempfile::TempDir;

/// A server sandboxed to a fresh temp directory, plus that canonical root.
fn server_at(dir: &TempDir) -> (FfmServer, PathBuf) {
    let root = dir.path().canonicalize().expect("canonicalize temp dir");
    (FfmServer::new(root.clone()), root)
}

/// The JSON payload a successful tool call carries in its text content.
fn tool_json(result: &CallToolResult) -> serde_json::Value {
    let text = result
        .content
        .first()
        .and_then(|content| content.as_text())
        .expect("a tool result carries one text content");
    serde_json::from_str(&text.text).expect("a tool payload is JSON")
}

fn write_file_at(path: &Path, content: &str) {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).expect("create parent directory");
    }
    fs::write(path, content).expect("write fixture file");
}

/// Assert that `path` is still a symlink.
///
/// `symlink_metadata`, never `Path::exists()`: `exists()` follows the link, so
/// it passes just as happily when the link has been replaced by a regular file
/// — or fails on a link that is merely dangling. Both make the assertion a lie.
#[cfg(unix)]
fn assert_is_symlink(path: &Path) {
    let meta = fs::symlink_metadata(path)
        .unwrap_or_else(|e| panic!("symlink_metadata {}: {e}", path.display()));
    assert!(
        meta.file_type().is_symlink(),
        "{} should still be a symlink",
        path.display()
    );
}

#[cfg(unix)]
fn assert_gone(path: &Path) {
    assert!(
        fs::symlink_metadata(path).is_err(),
        "{} should have been removed",
        path.display()
    );
}

/// A secret file in a tree the server root does not cover, plus a link to it
/// planted *inside* the root — the shape every "walk leaves the root" test
/// needs. Returns the outside directory, which must outlive the test.
#[cfg(unix)]
fn plant_link_out_of_root(root: &Path, link_name: &str, secret: &str) -> TempDir {
    let outside = TempDir::new().expect("temp dir");
    let target = outside.path().join("secret.txt");
    write_file_at(&target, secret);
    std::os::unix::fs::symlink(&target, root.join(link_name)).expect("plant the link");
    outside
}

// ═══════════════════════════════════════════════════════════════
// The registered tool list
// ═══════════════════════════════════════════════════════════════

#[test]
fn tool_names_match_the_registered_router() {
    let mut registered: Vec<String> = FfmServer::tool_router()
        .list_all()
        .into_iter()
        .map(|tool| tool.name.into_owned())
        .collect();
    registered.sort();

    let mut declared: Vec<String> = TOOL_NAMES.iter().map(|name| (*name).to_owned()).collect();
    declared.sort();

    assert_eq!(
        TOOL_NAMES.len(),
        registered.len(),
        "TOOL_NAMES is what a status screen counts — it must match the router exactly"
    );
    assert_eq!(
        declared, registered,
        "TOOL_NAMES has drifted from the tools the router registers"
    );
}

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
        .expect("a batch reports per-path failures instead of aborting");
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
        .expect("a batch reports per-path failures instead of aborting");

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

// ═══════════════════════════════════════════════════════════════
// Recursive walkers never leave the root through a planted link
// ═══════════════════════════════════════════════════════════════

#[cfg(unix)]
#[test]
fn grep_does_not_read_through_a_link_out_of_the_root() {
    let dir = TempDir::new().expect("temp dir");
    let (server, root) = server_at(&dir);
    let _outside = plant_link_out_of_root(&root, "creds", "aws_secret_access_key = hunter2");

    let result = server
        .grep(Parameters(GrepParams {
            path: ".".to_owned(),
            pattern: "aws_secret_access_key".to_owned(),
            extension: None,
            max_results: 50,
        }))
        .expect("grep runs");
    let payload = tool_json(&result);

    assert_eq!(
        payload["count"], 0,
        "the walk must not open a link that names a file outside the root, got: {payload}"
    );
}

#[cfg(unix)]
#[test]
fn replace_does_not_read_through_a_link_out_of_the_root() {
    let dir = TempDir::new().expect("temp dir");
    let (server, root) = server_at(&dir);
    let outside = plant_link_out_of_root(&root, "creds", "aws_secret_access_key = hunter2");

    let result = server
        .replace(Parameters(ReplaceParams {
            path: ".".to_owned(),
            find: "aws_secret_access_key".to_owned(),
            replace: "REDACTED".to_owned(),
            extension: None,
            name_pattern: None,
            dry_run: false,
        }))
        .expect("replace runs");
    let payload = tool_json(&result);

    assert_eq!(payload["files_affected"], 0);
    assert_eq!(
        payload["errors"], 0,
        "a link is stepped over, not opened and then refused, got: {payload}"
    );
    assert_eq!(
        fs::read_to_string(outside.path().join("secret.txt")).expect("the outside file survives"),
        "aws_secret_access_key = hunter2",
        "nothing outside the root may be rewritten"
    );
}

#[cfg(unix)]
#[test]
fn replace_dry_run_does_not_count_matches_behind_a_link() {
    let dir = TempDir::new().expect("temp dir");
    let (server, root) = server_at(&dir);
    let _outside = plant_link_out_of_root(&root, "creds", "aws_secret_access_key = hunter2");

    let result = server
        .replace(Parameters(ReplaceParams {
            path: ".".to_owned(),
            find: "aws_secret_access_key".to_owned(),
            replace: "REDACTED".to_owned(),
            extension: None,
            name_pattern: None,
            dry_run: true,
        }))
        .expect("replace runs");
    let payload = tool_json(&result);

    assert_eq!(
        payload["total_replacements"], 0,
        "a preview that counts matches is a preview that read the file, got: {payload}"
    );
}

#[cfg(unix)]
#[test]
fn glob_does_not_report_a_link_out_of_the_root() {
    let dir = TempDir::new().expect("temp dir");
    let (server, root) = server_at(&dir);
    let _outside = plant_link_out_of_root(&root, "creds", "aws_secret_access_key = hunter2");
    write_file_at(&root.join("inside.txt"), "public");

    let result = server
        .glob(Parameters(GlobParams {
            path: ".".to_owned(),
            pattern: "*".to_owned(),
            max_results: 100,
        }))
        .expect("glob runs");
    let payload = tool_json(&result);

    assert_eq!(
        payload["count"], 1,
        "only the real entry may be listed, got: {payload}"
    );
    assert_eq!(payload["results"][0]["name"], "inside.txt");
}

#[cfg(unix)]
#[test]
fn delete_by_pattern_leaves_a_matching_link_alone() {
    let dir = TempDir::new().expect("temp dir");
    let (server, root) = server_at(&dir);
    let _outside = plant_link_out_of_root(&root, "creds.tmp", "secret");

    let result = server
        .delete_by_pattern(Parameters(DeleteByPatternParams {
            path: ".".to_owned(),
            pattern: "*.tmp".to_owned(),
            include_dirs: false,
            recursive: true,
            dry_run: false,
        }))
        .expect("delete_by_pattern runs");
    let payload = tool_json(&result);

    assert_eq!(
        payload["deleted_count"], 0,
        "a pattern matches files, not the links that happen to share their names"
    );
    assert_is_symlink(&root.join("creds.tmp"));
}

#[cfg(unix)]
#[test]
fn disk_usage_does_not_total_a_tree_outside_the_root() {
    let dir = TempDir::new().expect("temp dir");
    let (server, root) = server_at(&dir);
    let outside = TempDir::new().expect("temp dir");
    write_file_at(&outside.path().join("big.bin"), &"x".repeat(64 * 1024));
    std::os::unix::fs::symlink(outside.path(), root.join("link")).expect("symlink");

    let result = server
        .disk_usage(Parameters(PathParam {
            path: ".".to_owned(),
        }))
        .expect("disk usage runs");
    let size = tool_json(&result)["size"]
        .as_u64()
        .expect("a size is reported");

    assert!(
        size < 64 * 1024,
        "following the link would have summed a tree outside the root, got {size} bytes"
    );
}

// ═══════════════════════════════════════════════════════════════
// A caller-supplied file name is one component, never a path
// ═══════════════════════════════════════════════════════════════

#[test]
fn unique_name_rejects_a_traversing_filename() {
    let dir = TempDir::new().expect("temp dir");
    let (server, _root) = server_at(&dir);

    let error = server
        .unique_name(Parameters(UniqueNameParams {
            directory: ".".to_owned(),
            filename: "../../../../etc/passwd".to_owned(),
        }))
        // Answering at all leaks whether the file is there: an existing target
        // comes back as "passwd_1", a missing one as "passwd".
        .expect_err("a file name may not walk out of the directory it is for");

    assert!(
        error.message.contains("plain file name"),
        "got: {}",
        error.message
    );
}

#[test]
fn unique_name_rejects_an_absolute_filename() {
    let dir = TempDir::new().expect("temp dir");
    let (server, _root) = server_at(&dir);

    let error = server
        .unique_name(Parameters(UniqueNameParams {
            directory: ".".to_owned(),
            // `Path::join` discards the base outright for an absolute operand.
            filename: "/etc/passwd".to_owned(),
        }))
        .expect_err("an absolute name is not a name");

    assert!(
        error.message.contains("plain file name"),
        "got: {}",
        error.message
    );
}

#[test]
fn join_child_accepts_only_one_ordinary_component() {
    let base = Path::new("/srv/root");

    assert_eq!(
        join_child(base, "report.txt").expect("a plain name is fine"),
        base.join("report.txt")
    );
    for hostile in ["../escape", "sub/dir.txt", "/etc/passwd", "..", ".", ""] {
        assert!(
            join_child(base, hostile).is_err(),
            "'{hostile}' must not be joinable onto a confined directory"
        );
    }
}
