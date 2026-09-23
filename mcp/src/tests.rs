//! In-process tests for the MCP tool handlers.
//!
//! `FfmServer` needs no transport to exist and every `#[tool]` method is a
//! plain `&self` function returning `Result<CallToolResult, McpError>`, so the
//! handlers are driven directly here — no child process, no JSON-RPC framing.
//! The BDD suite covers the happy paths over the wire; these tests pin the
//! refusals, the filters and the failure handling that never reach it.

use super::*;
use std::time::{Duration, SystemTime};
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

/// The `name` field of every entry under `key`, in the order returned.
fn entry_names(payload: &serde_json::Value, key: &str) -> Vec<String> {
    payload[key]
        .as_array()
        .expect("the payload holds a list")
        .iter()
        .map(|entry| entry["name"].as_str().unwrap_or_default().to_owned())
        .collect()
}

fn write_file_at(path: &Path, content: &str) {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).expect("create parent directory");
    }
    fs::write(path, content).expect("write fixture file");
}

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
    }));
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

    let result = server.batch_delete(Parameters(BatchParams {
        paths: vec!["full".to_owned(), "empty".to_owned()],
        destination: None,
        recursive: false,
    }));
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

    assert!(!error.message.is_empty(), "the failure has to be described");
    assert_eq!(
        fs::read_to_string(root.join("doc.txt")).expect("the source is still readable"),
        "content",
        "the copy fallback failed, so the source must not have been removed"
    );
    assert!(root.join("blocked").join("child.txt").exists());
}

// ═══════════════════════════════════════════════════════════════
// unique_name
// ═══════════════════════════════════════════════════════════════

#[test]
fn unique_name_sidesteps_an_existing_file() {
    let dir = TempDir::new().expect("temp dir");
    let (server, root) = server_at(&dir);
    write_file_at(&root.join("report.txt"), "first");

    let result = server
        .unique_name(Parameters(UniqueNameParams {
            directory: root.display().to_string(),
            filename: "report.txt".to_owned(),
        }))
        .expect("a free name exists");
    let payload = tool_json(&result);

    assert_eq!(payload["unique_name"], "report_1.txt");
    assert_eq!(
        payload["unique_path"],
        root.join("report_1.txt").display().to_string()
    );
}

#[test]
fn unique_name_keeps_counting_past_an_occupied_suffix() {
    let dir = TempDir::new().expect("temp dir");
    let (server, root) = server_at(&dir);
    write_file_at(&root.join("report.txt"), "first");
    write_file_at(&root.join("report_1.txt"), "second");

    let result = server
        .unique_name(Parameters(UniqueNameParams {
            directory: root.display().to_string(),
            filename: "report.txt".to_owned(),
        }))
        .expect("a free name exists");

    assert_eq!(tool_json(&result)["unique_name"], "report_2.txt");
}

#[test]
fn unique_name_without_an_extension_just_appends_the_counter() {
    let dir = TempDir::new().expect("temp dir");
    let (server, root) = server_at(&dir);
    write_file_at(&root.join("README"), "first");

    let result = server
        .unique_name(Parameters(UniqueNameParams {
            directory: root.display().to_string(),
            filename: "README".to_owned(),
        }))
        .expect("a free name exists");

    assert_eq!(tool_json(&result)["unique_name"], "README_1");
}

#[test]
fn unique_name_rejects_a_file_used_as_the_directory() {
    let dir = TempDir::new().expect("temp dir");
    let (server, root) = server_at(&dir);
    write_file_at(&root.join("notes.txt"), "notes");

    let error = server
        .unique_name(Parameters(UniqueNameParams {
            directory: "notes.txt".to_owned(),
            filename: "report.txt".to_owned(),
        }))
        .expect_err("a file is not a directory to place a name in");

    assert!(
        error.message.contains("not a directory"),
        "got: {}",
        error.message
    );
}

// ═══════════════════════════════════════════════════════════════
// permissions
// ═══════════════════════════════════════════════════════════════

#[cfg(unix)]
fn mode_of(path: &Path) -> u32 {
    use std::os::unix::fs::PermissionsExt;
    fs::metadata(path).expect("metadata").permissions().mode() & 0o7777
}

#[cfg(unix)]
#[test]
fn permissions_applies_an_octal_mode() {
    use std::os::unix::fs::PermissionsExt;
    let dir = TempDir::new().expect("temp dir");
    let (server, root) = server_at(&dir);
    let file = root.join("key");
    write_file_at(&file, "secret");
    fs::set_permissions(&file, fs::Permissions::from_mode(0o644)).expect("chmod");

    let result = server
        .permissions(Parameters(PermissionsParams {
            path: "key".to_owned(),
            mode: Some("600".to_owned()),
            readonly: None,
        }))
        .expect("setting a mode succeeds");

    assert_eq!(mode_of(&file), 0o600);
    assert!(
        tool_json(&result)["mode"]
            .as_str()
            .expect("the payload reports a mode")
            .ends_with("600"),
        "the payload must report what is on disk"
    );
}

#[cfg(unix)]
#[test]
fn permissions_rejects_a_non_octal_mode_and_changes_nothing() {
    use std::os::unix::fs::PermissionsExt;
    let dir = TempDir::new().expect("temp dir");
    let (server, root) = server_at(&dir);
    let file = root.join("key");
    write_file_at(&file, "secret");
    fs::set_permissions(&file, fs::Permissions::from_mode(0o600)).expect("chmod");

    let error = server
        .permissions(Parameters(PermissionsParams {
            path: "key".to_owned(),
            mode: Some("rwx".to_owned()),
            readonly: None,
        }))
        .expect_err("'rwx' is not an octal mode");

    assert!(
        error.message.contains("invalid octal mode"),
        "got: {}",
        error.message
    );
    assert_eq!(mode_of(&file), 0o600, "a rejected mode must not be applied");
}

#[cfg(unix)]
#[test]
fn permissions_rejects_a_mode_above_the_octal_maximum() {
    use std::os::unix::fs::PermissionsExt;
    let dir = TempDir::new().expect("temp dir");
    let (server, root) = server_at(&dir);
    let file = root.join("key");
    write_file_at(&file, "secret");
    fs::set_permissions(&file, fs::Permissions::from_mode(0o600)).expect("chmod");

    // 0o10000 parses fine but would silently truncate to 0 once handed to
    // `from_mode`, stripping every permission bit from the file.
    let error = server
        .permissions(Parameters(PermissionsParams {
            path: "key".to_owned(),
            mode: Some("10000".to_owned()),
            readonly: None,
        }))
        .expect_err("a mode wider than 7777 is out of range");

    assert!(
        error.message.contains("exceeds maximum 7777"),
        "got: {}",
        error.message
    );
    assert_eq!(mode_of(&file), 0o600, "a rejected mode must not be applied");
}

#[cfg(unix)]
#[test]
fn permissions_applies_mode_after_readonly_so_the_explicit_mode_wins() {
    use std::os::unix::fs::PermissionsExt;
    let dir = TempDir::new().expect("temp dir");
    let (server, root) = server_at(&dir);
    let file = root.join("key");
    write_file_at(&file, "secret");
    fs::set_permissions(&file, fs::Permissions::from_mode(0o644)).expect("chmod");

    // readonly=true alone would land on 0o444; running it *after* the mode
    // would leave 0o400. The caller asked for 0o600, so 0o600 is what must
    // survive.
    let result = server
        .permissions(Parameters(PermissionsParams {
            path: "key".to_owned(),
            mode: Some("600".to_owned()),
            readonly: Some(true),
        }))
        .expect("both fields together succeed");

    assert_eq!(mode_of(&file), 0o600);
    assert_eq!(
        tool_json(&result)["readonly"],
        false,
        "the payload is re-read from disk, where owner write is back"
    );
}

#[cfg(unix)]
#[test]
fn permissions_readonly_clears_every_write_bit() {
    use std::os::unix::fs::PermissionsExt;
    let dir = TempDir::new().expect("temp dir");
    let (server, root) = server_at(&dir);
    let file = root.join("shared");
    write_file_at(&file, "data");
    fs::set_permissions(&file, fs::Permissions::from_mode(0o664)).expect("chmod");

    let result = server
        .permissions(Parameters(PermissionsParams {
            path: "shared".to_owned(),
            mode: None,
            readonly: Some(true),
        }))
        .expect("setting readonly succeeds");

    assert_eq!(mode_of(&file), 0o444, "group and world write must go too");
    assert_eq!(tool_json(&result)["readonly"], true);
}

// ═══════════════════════════════════════════════════════════════
// replace
// ═══════════════════════════════════════════════════════════════

#[test]
fn replace_rejects_a_file_as_the_search_root() {
    let dir = TempDir::new().expect("temp dir");
    let (server, root) = server_at(&dir);
    write_file_at(&root.join("doc.txt"), "old");

    let error = server
        .replace(Parameters(ReplaceParams {
            path: "doc.txt".to_owned(),
            find: "old".to_owned(),
            replace: "new".to_owned(),
            extension: None,
            name_pattern: None,
            dry_run: false,
        }))
        .expect_err("replace walks a tree, not a single file");

    assert!(
        error.message.contains("not a directory"),
        "got: {}",
        error.message
    );
}

#[test]
fn replace_rejects_an_empty_find_pattern() {
    let dir = TempDir::new().expect("temp dir");
    let (server, root) = server_at(&dir);
    write_file_at(&root.join("doc.txt"), "keep");

    // `"keep".matches("")` counts five empty matches, so without this guard the
    // file would be rewritten with the replacement spliced between every char.
    let error = server
        .replace(Parameters(ReplaceParams {
            path: root.display().to_string(),
            find: String::new(),
            replace: "X".to_owned(),
            extension: None,
            name_pattern: None,
            dry_run: false,
        }))
        .expect_err("an empty needle matches everywhere");

    assert!(
        error.message.contains("find pattern must not be empty"),
        "got: {}",
        error.message
    );
    assert_eq!(
        fs::read_to_string(root.join("doc.txt")).expect("read"),
        "keep"
    );
}

#[test]
fn replace_extension_filter_leaves_other_extensions_alone() {
    let dir = TempDir::new().expect("temp dir");
    let (server, root) = server_at(&dir);
    write_file_at(&root.join("code.rs"), "old");
    write_file_at(&root.join("notes.txt"), "old");

    let result = server
        .replace(Parameters(ReplaceParams {
            path: root.display().to_string(),
            find: "old".to_owned(),
            replace: "new".to_owned(),
            extension: Some("rs".to_owned()),
            name_pattern: None,
            dry_run: false,
        }))
        .expect("replace runs");

    assert_eq!(tool_json(&result)["files_affected"], 1);
    assert_eq!(
        fs::read_to_string(root.join("code.rs")).expect("read"),
        "new"
    );
    assert_eq!(
        fs::read_to_string(root.join("notes.txt")).expect("read"),
        "old",
        "the extension filter must exclude, not merely order"
    );
}

#[test]
fn replace_name_pattern_filter_leaves_other_names_alone() {
    let dir = TempDir::new().expect("temp dir");
    let (server, root) = server_at(&dir);
    write_file_at(&root.join("keep_me.txt"), "old");
    write_file_at(&root.join("other.txt"), "old");

    let result = server
        .replace(Parameters(ReplaceParams {
            path: root.display().to_string(),
            find: "old".to_owned(),
            replace: "new".to_owned(),
            extension: None,
            name_pattern: Some("keep_*".to_owned()),
            dry_run: false,
        }))
        .expect("replace runs");

    assert_eq!(tool_json(&result)["files_affected"], 1);
    assert_eq!(
        fs::read_to_string(root.join("keep_me.txt")).expect("read"),
        "new"
    );
    assert_eq!(
        fs::read_to_string(root.join("other.txt")).expect("read"),
        "old",
        "the name filter must exclude, not merely order"
    );
}

// ═══════════════════════════════════════════════════════════════
// glob / tree / search
// ═══════════════════════════════════════════════════════════════

#[test]
fn glob_rejects_a_file_as_the_search_root() {
    let dir = TempDir::new().expect("temp dir");
    let (server, root) = server_at(&dir);
    write_file_at(&root.join("doc.txt"), "x");

    let error = server
        .glob(Parameters(GlobParams {
            path: "doc.txt".to_owned(),
            pattern: "*".to_owned(),
            max_results: 10,
        }))
        .expect_err("glob walks a tree, not a single file");

    assert!(
        error.message.contains("not a directory"),
        "got: {}",
        error.message
    );
}

#[test]
fn glob_stops_at_max_results() {
    let dir = TempDir::new().expect("temp dir");
    let (server, root) = server_at(&dir);
    write_file_at(&root.join("a.txt"), "a");
    write_file_at(&root.join("b.txt"), "b");
    write_file_at(&root.join("c.txt"), "c");

    let result = server
        .glob(Parameters(GlobParams {
            path: root.display().to_string(),
            pattern: "*.txt".to_owned(),
            max_results: 2,
        }))
        .expect("glob runs");

    assert_eq!(tool_json(&result)["count"], 2);
}

#[test]
fn glob_honours_max_results_across_a_recursive_descent() {
    let dir = TempDir::new().expect("temp dir");
    let (server, root) = server_at(&dir);
    // One directory holding one file: `**` matches the directory first, which
    // fills the budget, so the walk must not step inside it and report the
    // leaf as well — the cap has to survive crossing a directory boundary.
    write_file_at(&root.join("branch").join("leaf.txt"), "leaf");

    let result = server
        .glob(Parameters(GlobParams {
            path: root.display().to_string(),
            pattern: "**".to_owned(),
            max_results: 1,
        }))
        .expect("glob runs");
    let payload = tool_json(&result);

    assert_eq!(payload["count"], 1);
    assert_eq!(payload["results"][0]["relative_path"], "branch");
}

#[test]
fn tree_rejects_a_file_as_the_root() {
    let dir = TempDir::new().expect("temp dir");
    let (server, root) = server_at(&dir);
    write_file_at(&root.join("doc.txt"), "x");

    let error = server
        .tree(Parameters(TreeParams {
            path: "doc.txt".to_owned(),
            depth: 2,
        }))
        .expect_err("a tree needs a directory to walk");

    assert!(
        error.message.contains("not a directory"),
        "got: {}",
        error.message
    );
}

#[test]
fn search_honours_max_results_across_a_recursive_descent() {
    let dir = TempDir::new().expect("temp dir");
    let (server, root) = server_at(&dir);
    // The directory matches the pattern too, so it fills the single result
    // slot and the walk must stop before reaching the file inside it.
    write_file_at(&root.join("match_dir").join("match_file.txt"), "x");

    let result = server
        .search(Parameters(SearchParams {
            path: root.display().to_string(),
            pattern: "match".to_owned(),
            max_results: 1,
            files_only: false,
            dirs_only: false,
        }))
        .expect("search runs");
    let payload = tool_json(&result);

    assert_eq!(payload["count"], 1);
    assert_eq!(entry_names(&payload, "results"), vec!["match_dir"]);
}

// ═══════════════════════════════════════════════════════════════
// find filters
// ═══════════════════════════════════════════════════════════════

#[test]
fn find_excludes_files_over_the_maximum_size() {
    let dir = TempDir::new().expect("temp dir");
    let (server, root) = server_at(&dir);
    write_file_at(&root.join("small.txt"), "abc");
    write_file_at(&root.join("large.txt"), "abcdefghijkl");

    let result = server
        .find(Parameters(FindParams {
            path: root.display().to_string(),
            name: None,
            file_type: None,
            min_size: None,
            max_size: Some(3),
            max_depth: None,
            modified_within_secs: None,
            max_results: 100,
        }))
        .expect("find runs");
    let payload = tool_json(&result);

    assert_eq!(payload["count"], 1);
    assert_eq!(entry_names(&payload, "results"), vec!["small.txt"]);
}

#[test]
fn find_excludes_files_modified_before_the_time_window() {
    let dir = TempDir::new().expect("temp dir");
    let (server, root) = server_at(&dir);
    write_file_at(&root.join("fresh.txt"), "new");
    let stale = root.join("stale.txt");
    write_file_at(&stale, "old");
    // Backdate the modification time so the window has something to exclude.
    let handle = fs::OpenOptions::new()
        .write(true)
        .open(&stale)
        .expect("open for set_modified");
    handle
        .set_modified(SystemTime::now() - Duration::from_secs(3600))
        .expect("backdate the modification time");
    drop(handle);

    let result = server
        .find(Parameters(FindParams {
            path: root.display().to_string(),
            name: None,
            file_type: None,
            min_size: None,
            max_size: None,
            max_depth: None,
            modified_within_secs: Some(60),
            max_results: 100,
        }))
        .expect("find runs");
    let payload = tool_json(&result);

    assert_eq!(payload["count"], 1);
    assert_eq!(entry_names(&payload, "results"), vec!["fresh.txt"]);
}

#[test]
fn find_rejects_a_file_as_the_root() {
    let dir = TempDir::new().expect("temp dir");
    let (server, root) = server_at(&dir);
    write_file_at(&root.join("doc.txt"), "x");

    let error = server
        .find(Parameters(FindParams {
            path: "doc.txt".to_owned(),
            name: None,
            file_type: None,
            min_size: None,
            max_size: None,
            max_depth: None,
            modified_within_secs: None,
            max_results: 100,
        }))
        .expect_err("find walks a tree, not a single file");

    assert!(
        error.message.contains("not a directory"),
        "got: {}",
        error.message
    );
}

// ═══════════════════════════════════════════════════════════════
// delete_by_pattern — directories
// ═══════════════════════════════════════════════════════════════

#[test]
fn delete_by_pattern_leaves_matching_directories_alone_by_default() {
    let dir = TempDir::new().expect("temp dir");
    let (server, root) = server_at(&dir);
    write_file_at(&root.join("cache.tmp").join("inner.txt"), "inner");

    let result = server
        .delete_by_pattern(Parameters(DeleteByPatternParams {
            path: root.display().to_string(),
            pattern: "*.tmp".to_owned(),
            include_dirs: false,
            recursive: true,
            dry_run: false,
        }))
        .expect("cleanup runs");

    assert_eq!(tool_json(&result)["deleted_count"], 0);
    assert!(
        root.join("cache.tmp").join("inner.txt").exists(),
        "a directory must not be swept up by a file pattern unless asked"
    );
}

#[test]
fn delete_by_pattern_removes_a_matching_directory_when_include_dirs_is_set() {
    let dir = TempDir::new().expect("temp dir");
    let (server, root) = server_at(&dir);
    write_file_at(&root.join("cache.tmp").join("inner.txt"), "12345");
    write_file_at(&root.join("keep.txt"), "keep");

    let result = server
        .delete_by_pattern(Parameters(DeleteByPatternParams {
            path: root.display().to_string(),
            pattern: "*.tmp".to_owned(),
            include_dirs: true,
            recursive: true,
            dry_run: false,
        }))
        .expect("cleanup runs");
    let payload = tool_json(&result);

    assert_eq!(payload["deleted_count"], 1);
    assert_eq!(payload["deleted"][0]["type"], "dir");
    assert_eq!(
        payload["deleted"][0]["size"], 5,
        "the reported size is the tree's, measured before the removal"
    );
    assert!(!root.join("cache.tmp").exists());
    assert!(root.join("keep.txt").exists());
}

#[test]
fn delete_by_pattern_dry_run_reports_a_directory_without_removing_it() {
    let dir = TempDir::new().expect("temp dir");
    let (server, root) = server_at(&dir);
    write_file_at(&root.join("cache.tmp").join("inner.txt"), "inner");

    let result = server
        .delete_by_pattern(Parameters(DeleteByPatternParams {
            path: root.display().to_string(),
            pattern: "*.tmp".to_owned(),
            include_dirs: true,
            recursive: true,
            dry_run: true,
        }))
        .expect("cleanup runs");
    let payload = tool_json(&result);

    assert_eq!(payload["deleted_count"], 1);
    assert_eq!(payload["deleted"][0]["would_delete"], true);
    assert!(
        root.join("cache.tmp").join("inner.txt").exists(),
        "a dry run must not touch the tree"
    );
}

#[test]
fn delete_by_pattern_rejects_a_file_as_the_root() {
    let dir = TempDir::new().expect("temp dir");
    let (server, root) = server_at(&dir);
    write_file_at(&root.join("doc.txt"), "x");

    let error = server
        .delete_by_pattern(Parameters(DeleteByPatternParams {
            path: "doc.txt".to_owned(),
            pattern: "*".to_owned(),
            include_dirs: false,
            recursive: true,
            dry_run: false,
        }))
        .expect_err("cleanup walks a tree, not a single file");

    assert!(
        error.message.contains("not a directory"),
        "got: {}",
        error.message
    );
}

// ═══════════════════════════════════════════════════════════════
// grep
// ═══════════════════════════════════════════════════════════════

#[test]
fn grep_descends_into_subdirectories() {
    let dir = TempDir::new().expect("temp dir");
    let (server, root) = server_at(&dir);
    write_file_at(
        &root.join("deep").join("nested.txt"),
        "first\nNEEDLE here\n",
    );

    let result = server
        .grep(Parameters(GrepParams {
            path: root.display().to_string(),
            pattern: "needle".to_owned(),
            extension: None,
            max_results: 50,
        }))
        .expect("grep runs");
    let payload = tool_json(&result);

    assert_eq!(payload["count"], 1);
    assert_eq!(
        payload["matches"][0]["file"],
        root.join("deep").join("nested.txt").display().to_string()
    );
    assert_eq!(payload["matches"][0]["line"], 2);
    assert_eq!(payload["matches"][0]["content"], "NEEDLE here");
}

#[test]
fn grep_stops_at_max_results_inside_one_file() {
    let dir = TempDir::new().expect("temp dir");
    let (server, root) = server_at(&dir);
    write_file_at(&root.join("log.txt"), "needle 1\nneedle 2\nneedle 3\n");

    let result = server
        .grep(Parameters(GrepParams {
            path: root.display().to_string(),
            pattern: "needle".to_owned(),
            extension: None,
            max_results: 2,
        }))
        .expect("grep runs");

    assert_eq!(tool_json(&result)["count"], 2);
}

// ═══════════════════════════════════════════════════════════════
// disk_usage / view_file / format_size
// ═══════════════════════════════════════════════════════════════

#[test]
fn disk_usage_reports_a_missing_path_instead_of_zero() {
    let dir = TempDir::new().expect("temp dir");
    let (server, _root) = server_at(&dir);

    let error = server
        .disk_usage(Parameters(PathParam {
            path: "ghost".to_owned(),
        }))
        .expect_err("a size of 0 would read as an empty directory");

    assert!(
        error.message.contains("not found"),
        "got: {}",
        error.message
    );
}

#[test]
fn disk_usage_of_a_single_file_is_its_own_size() {
    let dir = TempDir::new().expect("temp dir");
    let (server, root) = server_at(&dir);
    write_file_at(&root.join("doc.txt"), "abcde");

    let result = server
        .disk_usage(Parameters(PathParam {
            path: "doc.txt".to_owned(),
        }))
        .expect("disk usage runs");

    assert_eq!(tool_json(&result)["size"], 5);
}

#[test]
fn view_file_refuses_a_file_over_the_size_limit() {
    let dir = TempDir::new().expect("temp dir");
    let (server, root) = server_at(&dir);
    let big = root.join("big.bin");
    // Sparse: the file reports 11 MB without 11 MB ever being written.
    let handle = fs::File::create(&big).expect("create");
    handle.set_len(11 * 1024 * 1024).expect("grow sparsely");
    drop(handle);

    let error = server
        .view_file(Parameters(ViewFileParams {
            path: "big.bin".to_owned(),
            offset: None,
            limit: None,
        }))
        .expect_err("11 MB is over the 10 MB ceiling");

    assert!(
        error.message.contains("file too large"),
        "got: {}",
        error.message
    );
    assert!(
        error.message.contains("11.00 MB"),
        "the refusal should say how big the file actually is, got: {}",
        error.message
    );
}

#[test]
fn format_size_switches_to_gigabytes() {
    let result = FfmServer::format_size_tool(Parameters(FormatSizeParams {
        bytes: 3 * 1024 * 1024 * 1024,
    }));

    assert_eq!(tool_json(&result)["formatted"], "3.00 GB");
}
