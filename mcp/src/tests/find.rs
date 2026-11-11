//! Purpose: `find` filters by size and modification time and refuses a file root.

use super::*;
use crate::tools::patterns::FindParams;
use std::fs::OpenOptions;
use std::time::{Duration, SystemTime};

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
    let handle = OpenOptions::new()
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

#[test]
fn glob_and_find_reject_oversized_patterns() {
    let dir = TempDir::new().expect("temp dir");
    let (server, root) = server_at(&dir);
    let too_long = "a".repeat(globmatch::MAX_PATTERN_LEN + 1);

    let glob_err = server
        .glob(Parameters(GlobParams {
            path: root.display().to_string(),
            pattern: too_long.clone(),
            max_results: 10,
        }))
        .expect_err("oversized glob pattern must be rejected");
    assert!(glob_err.message.contains("glob pattern"));

    let find_err = server
        .find(Parameters(FindParams {
            path: root.display().to_string(),
            name: Some(too_long),
            file_type: None,
            min_size: None,
            max_size: None,
            max_depth: None,
            modified_within_secs: None,
            max_results: 10,
        }))
        .expect_err("oversized find name pattern must be rejected");
    assert!(find_err.message.contains("glob pattern"));
}
