//! Purpose: `delete_by_pattern` and directories: left alone unless asked, and
//! only reported in a dry run.

use super::*;

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
