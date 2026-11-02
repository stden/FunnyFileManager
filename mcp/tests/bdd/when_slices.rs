//! Purpose: `When` steps for line-slice and text-inspection tools:
//! `head_file`, `tail_file`, `read_lines`, `file_stats`, and `text_format`.

use crate::world::McpWorld;
use cucumber::when;
use serde_json::json;

#[when(expr = "I read the first {int} lines of {string}")]
async fn when_head_file(w: &mut McpWorld, lines: usize, name: String) {
    let path = w.resolve(&name);
    w.call_tool("head_file", json!({"path": path, "lines": lines}));
}

#[when(expr = "I read the last {int} lines of {string}")]
async fn when_tail_file(w: &mut McpWorld, lines: usize, name: String) {
    let path = w.resolve(&name);
    w.call_tool("tail_file", json!({"path": path, "lines": lines}));
}

#[when(expr = "I read lines {int} to {int} of {string}")]
async fn when_read_lines(w: &mut McpWorld, from: usize, to: usize, name: String) {
    let path = w.resolve(&name);
    w.call_tool("read_lines", json!({"path": path, "from": from, "to": to}));
}

#[when(expr = "I count text statistics of {string}")]
async fn when_file_stats(w: &mut McpWorld, name: String) {
    let path = w.resolve(&name);
    w.call_tool("file_stats", json!({"path": path}));
}

#[when(expr = "I inspect text format of {string}")]
async fn when_text_format(w: &mut McpWorld, name: String) {
    let path = w.resolve(&name);
    w.call_tool("text_format", json!({"path": path}));
}
