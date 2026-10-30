//! Purpose: `When` steps that call the pattern tools: glob, replace,
//! search, grep, find and delete-by-pattern.

use crate::world::McpWorld;
use cucumber::when;
use serde_json::json;

#[when(expr = "I glob {string}")]
async fn when_glob(w: &mut McpWorld, pattern: String) {
    let path = w.temp_path().display().to_string();
    w.call_tool("glob", json!({"path": path, "pattern": pattern}));
}

#[when(expr = "I replace {string} with {string}")]
async fn when_replace(w: &mut McpWorld, find: String, replace: String) {
    let path = w.temp_path().display().to_string();
    w.call_tool(
        "replace",
        json!({"path": path, "find": find, "replace": replace}),
    );
}

#[when(expr = "I search for {string}")]
async fn when_search(w: &mut McpWorld, pattern: String) {
    let path = w.temp_path().display().to_string();
    w.call_tool("search", json!({"path": path, "pattern": pattern}));
}

#[when(expr = "I search for {string} with max results {int}")]
async fn when_search_max(w: &mut McpWorld, pattern: String, max: usize) {
    let path = w.temp_path().display().to_string();
    w.call_tool(
        "search",
        json!({"path": path, "pattern": pattern, "max_results": max}),
    );
}

#[when(expr = "I search for {string} files only")]
async fn when_search_files(w: &mut McpWorld, pattern: String) {
    let path = w.temp_path().display().to_string();
    w.call_tool(
        "search",
        json!({"path": path, "pattern": pattern, "files_only": true}),
    );
}

#[when(expr = "I search for {string} dirs only")]
async fn when_search_dirs(w: &mut McpWorld, pattern: String) {
    let path = w.temp_path().display().to_string();
    w.call_tool(
        "search",
        json!({"path": path, "pattern": pattern, "dirs_only": true}),
    );
}

#[when(expr = "I grep for {string}")]
async fn when_grep(w: &mut McpWorld, pattern: String) {
    let path = w.temp_path().display().to_string();
    w.call_tool("grep", json!({"path": path, "pattern": pattern}));
}

#[when(expr = "I grep for {string} in {string} files")]
async fn when_grep_ext(w: &mut McpWorld, pattern: String, extension: String) {
    let path = w.temp_path().display().to_string();
    w.call_tool(
        "grep",
        json!({"path": path, "pattern": pattern, "extension": extension}),
    );
}

#[when(expr = "I find entries named {string}")]
async fn when_find_named(w: &mut McpWorld, name: String) {
    let path = w.temp_path().display().to_string();
    w.call_tool("find", json!({"path": path, "name": name}));
}

#[when(expr = "I find entries of type {string}")]
async fn when_find_type(w: &mut McpWorld, file_type: String) {
    let path = w.temp_path().display().to_string();
    w.call_tool("find", json!({"path": path, "file_type": file_type}));
}

#[when(expr = "I find entries of at least {int} bytes")]
async fn when_find_min_size(w: &mut McpWorld, min_size: u64) {
    let path = w.temp_path().display().to_string();
    w.call_tool(
        "find",
        json!({"path": path, "file_type": "file", "min_size": min_size}),
    );
}

#[when(expr = "I find entries no deeper than {int}")]
async fn when_find_max_depth(w: &mut McpWorld, max_depth: usize) {
    let path = w.temp_path().display().to_string();
    w.call_tool(
        "find",
        json!({"path": path, "file_type": "file", "max_depth": max_depth}),
    );
}

#[when(expr = "I delete files matching {string}")]
async fn when_delete_pattern(w: &mut McpWorld, pattern: String) {
    let path = w.temp_path().display().to_string();
    w.call_tool(
        "delete_by_pattern",
        json!({"path": path, "pattern": pattern}),
    );
}

#[when(expr = "I preview deleting files matching {string}")]
async fn when_preview_delete_pattern(w: &mut McpWorld, pattern: String) {
    let path = w.temp_path().display().to_string();
    w.call_tool(
        "delete_by_pattern",
        json!({"path": path, "pattern": pattern, "dry_run": true}),
    );
}

#[when(expr = "I preview replacing {string} with {string}")]
async fn when_preview_replace(w: &mut McpWorld, find: String, replace: String) {
    let path = w.temp_path().display().to_string();
    w.call_tool(
        "replace",
        json!({"path": path, "find": find, "replace": replace, "dry_run": true}),
    );
}
