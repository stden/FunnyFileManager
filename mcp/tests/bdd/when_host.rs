//! Purpose: `When` steps that call the host tools: drives, size formatting,
//! bookmarks, system info and AWS profiles.

use crate::world::McpWorld;
use cucumber::when;
use serde_json::json;

#[when("I list drives")]
async fn when_list_drives(w: &mut McpWorld) {
    w.call_tool("list_drives", json!({}));
}

#[when(expr = "I format {int} bytes")]
async fn when_format_size(w: &mut McpWorld, bytes: u64) {
    w.call_tool("format_size_tool", json!({"bytes": bytes}));
}

#[when(expr = "I bookmark the temporary directory as {string}")]
async fn when_set_bookmark(w: &mut McpWorld, name: String) {
    let path = w.temp_path().display().to_string();
    w.call_tool("set_bookmark", json!({"name": name, "path": path}));
}

#[when(expr = "I look up the bookmark {string}")]
async fn when_get_bookmark(w: &mut McpWorld, name: String) {
    w.call_tool("get_bookmark", json!({"name": name}));
}

#[when("I list bookmarks")]
async fn when_list_bookmarks(w: &mut McpWorld) {
    w.call_tool("list_bookmarks", json!({}));
}

#[when(expr = "I remove the bookmark {string}")]
async fn when_delete_bookmark(w: &mut McpWorld, name: String) {
    w.call_tool("delete_bookmark", json!({"name": name}));
}

#[when("I get system info")]
async fn when_system_info(w: &mut McpWorld) {
    w.call_tool("system_info", json!({}));
}

#[when("I list AWS profiles")]
async fn when_list_aws_profiles(w: &mut McpWorld) {
    w.call_tool("s3_list_profiles", json!({}));
}
