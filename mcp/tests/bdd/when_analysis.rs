//! Purpose: `When` steps for comparison, space analysis, duplicate detection,
//! and executable lookup tools.

use crate::world::McpWorld;
use cucumber::when;
use serde_json::json;

#[when(expr = "I compare file {string} with file {string}")]
async fn when_compare_files(w: &mut McpWorld, left: String, right: String) {
    let left_path = w.resolve(&left);
    let right_path = w.resolve(&right);
    w.call_tool(
        "compare_files",
        json!({"left": left_path, "right": right_path}),
    );
}

#[when(expr = "I compare directory {string} with directory {string}")]
async fn when_compare_directories(w: &mut McpWorld, left: String, right: String) {
    let left_path = w.resolve(&left);
    let right_path = w.resolve(&right);
    w.call_tool(
        "compare_directories",
        json!({"left": left_path, "right": right_path}),
    );
}

#[when("I analyse the temporary directory")]
async fn when_analyse_directory(w: &mut McpWorld) {
    let path = w.temp_path().display().to_string();
    w.call_tool("analyse_directory", json!({"path": path}));
}

#[when(expr = "I list the top {int} largest files")]
async fn when_largest_files(w: &mut McpWorld, limit: usize) {
    let path = w.temp_path().display().to_string();
    w.call_tool("largest_files", json!({"path": path, "limit": limit}));
}

#[when(expr = "I list files modified within {int} days")]
async fn when_recent_files(w: &mut McpWorld, days: u64) {
    let path = w.temp_path().display().to_string();
    w.call_tool("recent_files", json!({"path": path, "days": days}));
}

#[when("I search for duplicate files")]
async fn when_find_duplicates(w: &mut McpWorld) {
    let path = w.temp_path().display().to_string();
    w.call_tool("find_duplicates", json!({"path": path}));
}

#[when(expr = "I look up executable {string} on PATH")]
async fn when_which(w: &mut McpWorld, name: String) {
    w.call_tool("which", json!({"name": name}));
}
