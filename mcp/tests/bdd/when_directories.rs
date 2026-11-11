//! Purpose: `When` steps that call the directory tools: create, list,
//! disk usage, tree, unique names and sorted listings.

use crate::world::McpWorld;
use cucumber::when;
use serde_json::json;

#[when(expr = "I create directory {string}")]
async fn when_create_dir(w: &mut McpWorld, name: String) {
    let path = w.resolve(&name);
    w.call_tool("create_directory", json!({"path": path}));
}

#[when("I list the temporary directory")]
async fn when_list_dir(w: &mut McpWorld) {
    let path = w.temp_path().display().to_string();
    w.call_tool("list_directory", json!({"path": path}));
}

#[when(expr = "I try to list {string}")]
async fn when_try_list(w: &mut McpWorld, name: String) {
    if name.is_empty() {
        w.call_tool("list_directory", json!({"path": ""}));
    } else {
        let path = w.resolve(&name);
        w.call_tool("list_directory", json!({"path": path}));
    }
}

#[when("I try to list a path with null byte")]
async fn when_list_null(w: &mut McpWorld) {
    w.call_tool("list_directory", json!({"path": "/tmp/\0bad"}));
}

#[when(expr = "I try to list the absolute path {string}")]
async fn when_try_list_absolute(w: &mut McpWorld, path: String) {
    w.call_tool("list_directory", json!({"path": path}));
}

#[when(expr = "I get disk usage of the temporary directory")]
async fn when_disk_usage(w: &mut McpWorld) {
    let path = w.temp_path().display().to_string();
    w.call_tool("disk_usage", json!({"path": path}));
}

#[when(expr = "I get tree of the temporary directory with depth {int}")]
async fn when_tree(w: &mut McpWorld, depth: usize) {
    let path = w.temp_path().display().to_string();
    w.call_tool("tree", json!({"path": path, "depth": depth}));
}

#[when(expr = "I ask for a unique name for {string}")]
async fn when_unique_name(w: &mut McpWorld, filename: String) {
    let directory = w.temp_path().display().to_string();
    w.call_tool(
        "unique_name",
        json!({"directory": directory, "filename": filename}),
    );
}

#[when(expr = "I sort the temporary directory by {string}")]
async fn when_sort_directory(w: &mut McpWorld, sort: String) {
    let path = w.temp_path().display().to_string();
    w.call_tool("sort_directory", json!({"path": path, "sort": sort}));
}

#[when(expr = "I sort the file {string} by {string}")]
async fn when_sort_file(w: &mut McpWorld, name: String, sort: String) {
    let path = w.resolve(&name);
    w.call_tool("sort_directory", json!({"path": path, "sort": sort}));
}
