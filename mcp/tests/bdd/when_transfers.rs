//! Purpose: `When` steps that copy, move and batch-process entries.

use crate::world::McpWorld;
use cucumber::when;
use serde_json::json;

#[when(expr = "I copy {string} to {string}")]
async fn when_copy(w: &mut McpWorld, source: String, dest: String) {
    let source = w.resolve(&source);
    let dest = w.resolve(&dest);
    w.call_tool("copy", json!({"source": source, "destination": dest}));
}

#[when(expr = "I copy directory {string} to {string}")]
async fn when_copy_dir(w: &mut McpWorld, source: String, dest: String) {
    let source = w.resolve(&source);
    let dest = w.resolve(&dest);
    w.call_tool("copy", json!({"source": source, "destination": dest}));
}

#[when(expr = "I move {string} to {string}")]
async fn when_move(w: &mut McpWorld, source: String, dest: String) {
    let source = w.resolve(&source);
    let dest = w.resolve(&dest);
    w.call_tool(
        "move_rename",
        json!({"source": source, "destination": dest}),
    );
}

#[when(expr = "I try to copy {string} to {string}")]
async fn when_try_copy(w: &mut McpWorld, source: String, dest: String) {
    let source = w.resolve(&source);
    let dest = w.resolve(&dest);
    w.call_tool("copy", json!({"source": source, "destination": dest}));
}

#[when(expr = "I try to move {string} to {string}")]
async fn when_try_move(w: &mut McpWorld, source: String, dest: String) {
    let source = w.resolve(&source);
    let dest = w.resolve(&dest);
    w.call_tool(
        "move_rename",
        json!({"source": source, "destination": dest}),
    );
}

/// Comma-separated names, resolved against the scenario's temp directory.
fn resolve_list(w: &McpWorld, names: &str) -> Vec<String> {
    names.split(',').map(|n| w.resolve(n.trim())).collect()
}

#[when(expr = "I batch copy {string} into {string}")]
async fn when_batch_copy(w: &mut McpWorld, names: String, dest: String) {
    let paths = resolve_list(w, &names);
    let destination = w.resolve(&dest);
    w.call_tool(
        "batch_copy",
        json!({"paths": paths, "destination": destination}),
    );
}

#[when(expr = "I batch move {string} into {string}")]
async fn when_batch_move(w: &mut McpWorld, names: String, dest: String) {
    let paths = resolve_list(w, &names);
    let destination = w.resolve(&dest);
    w.call_tool(
        "batch_move",
        json!({"paths": paths, "destination": destination}),
    );
}

#[when(expr = "I batch delete {string}")]
async fn when_batch_delete(w: &mut McpWorld, names: String) {
    let paths = resolve_list(w, &names);
    w.call_tool("batch_delete", json!({"paths": paths}));
}
