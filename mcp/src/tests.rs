//! In-process tests for the MCP tool handlers.
//!
//! `FfmServer` needs no transport to exist and every `#[tool]` method is a
//! plain `&self` function returning `Result<CallToolResult, McpError>`, so the
//! handlers are driven directly here — no child process, no JSON-RPC framing.
//! The BDD suite covers the happy paths over the wire; these tests pin the
//! refusals, the filters and the failure handling that never reach it. The
//! helpers live here; the tests, one child module per behaviour.

use super::*;
use rmcp::handler::server::wrapper::Parameters;
use rmcp::model::CallToolResult;
use std::fs;
use std::path::Path;
use tempfile::TempDir;

/// A server sandboxed to a fresh temp directory, plus that canonical root.
fn server_at(dir: &TempDir) -> (FfmServer, PathBuf) {
    let root = dir.path().canonicalize().expect("canonicalize temp dir");
    (FfmServer::new(root.clone()), root)
}

/// The JSON payload a successful tool call carries in its text content.
fn tool_json(result: &CallToolResult) -> serde_json::Value {
    let text = result
        .content
        .first()
        .and_then(|content| content.as_text())
        .expect("a tool result carries one text content");
    serde_json::from_str(&text.text).expect("a tool payload is JSON")
}

/// The `name` field of every entry under `key`, in the order returned.
fn entry_names(payload: &serde_json::Value, key: &str) -> Vec<String> {
    payload[key]
        .as_array()
        .expect("the payload holds a list")
        .iter()
        .map(|entry| entry["name"].as_str().unwrap_or_default().to_owned())
        .collect()
}

fn write_file_at(path: &Path, content: &str) {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).expect("create parent directory");
    }
    fs::write(path, content).expect("write fixture file");
}

mod delete_by_pattern;
mod failed_transfers;
mod find;
#[cfg(unix)]
mod permissions;
mod refusals;
mod replace;
mod search;
mod sizes;
mod unique_name;
