//! Purpose: `glob`, `tree`, `search` and `grep`: roots they refuse and the result
//! limits they honour.

use super::*;
use crate::tools::search::{SearchParams, TreeParams};

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
