//! Purpose: `When` steps that call the single-file tools: create, write,
//! view, delete, permissions, info, append, checksum and symlinks.

use crate::world::McpWorld;
use cucumber::when;
use serde_json::json;

#[when(expr = "I create file {string} with content {string}")]
async fn when_create_file(w: &mut McpWorld, name: String, content: String) {
    let path = w.resolve(&name);
    w.call_tool("create_file", json!({"path": path, "content": content}));
}

#[when(expr = "I try to create file {string} with content {string}")]
async fn when_try_create_file(w: &mut McpWorld, name: String, content: String) {
    let path = w.resolve(&name);
    w.call_tool("create_file", json!({"path": path, "content": content}));
}

#[when(expr = "I write {string} to file {string}")]
async fn when_write_file(w: &mut McpWorld, content: String, name: String) {
    let path = w.resolve(&name);
    w.call_tool("write_file", json!({"path": path, "content": content}));
}

#[when(expr = "I view file {string}")]
async fn when_view_file(w: &mut McpWorld, name: String) {
    let path = w.resolve(&name);
    w.call_tool("view_file", json!({"path": path}));
}

#[when(expr = "I view file {string} with offset {int} and limit {int}")]
async fn when_view_file_offset(w: &mut McpWorld, name: String, offset: u64, limit: u64) {
    let path = w.resolve(&name);
    w.call_tool(
        "view_file",
        json!({"path": path, "offset": offset, "limit": limit}),
    );
}

#[when("I try to view the temporary directory")]
async fn when_view_dir(w: &mut McpWorld) {
    let path = w.temp_path().display().to_string();
    w.call_tool("view_file", json!({"path": path}));
}

#[when(expr = "I delete {string}")]
async fn when_delete(w: &mut McpWorld, name: String) {
    let path = w.resolve(&name);
    w.call_tool("delete", json!({"path": path}));
}

#[when(expr = "I try to delete {string}")]
async fn when_try_delete(w: &mut McpWorld, name: String) {
    let path = w.resolve(&name);
    w.call_tool("delete", json!({"path": path}));
}

#[when(expr = "I clear the readonly flag on {string}")]
async fn when_clear_readonly(w: &mut McpWorld, name: String) {
    let path = w.resolve(&name);
    w.call_tool("permissions", json!({"path": path, "readonly": false}));
}

#[when(expr = "I get info for {string}")]
async fn when_get_info(w: &mut McpWorld, name: String) {
    let path = w.resolve(&name);
    w.call_tool("get_file_info", json!({"path": path}));
}

#[when(expr = "I try to get info for {string}")]
async fn when_try_get_info(w: &mut McpWorld, name: String) {
    let path = w.resolve(&name);
    w.call_tool("get_file_info", json!({"path": path}));
}

#[when(expr = "I append {string} to file {string}")]
async fn when_append_file(w: &mut McpWorld, content: String, name: String) {
    let path = w.resolve(&name);
    w.call_tool("append_file", json!({"path": path, "content": content}));
}

#[when(expr = "I checksum {string}")]
async fn when_checksum(w: &mut McpWorld, name: String) {
    let path = w.resolve(&name);
    w.call_tool("file_checksum", json!({"path": path}));
}

#[when(expr = "I try to checksum {string}")]
async fn when_try_checksum(w: &mut McpWorld, name: String) {
    let path = w.resolve(&name);
    w.call_tool("file_checksum", json!({"path": path}));
}

#[when(expr = "I link {string} to {string}")]
async fn when_create_symlink(w: &mut McpWorld, link: String, target: String) {
    let link_path = w.resolve(&link);
    let target_path = w.resolve(&target);
    w.call_tool(
        "create_symlink",
        json!({"link": link_path, "target": target_path}),
    );
}

#[when(expr = "I read the link {string}")]
async fn when_read_symlink(w: &mut McpWorld, name: String) {
    let path = w.resolve(&name);
    w.call_tool("read_symlink", json!({"path": path}));
}
