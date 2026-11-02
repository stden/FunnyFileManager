//! Purpose: One BDD scenario's state — its temp root, the tree outside that
//! root and the outcome of the last tool call — and how a JSON-RPC response
//! becomes that outcome.

use serde_json::{json, Value};
use std::path::PathBuf;
use std::process::ExitStatus;
use tempfile::TempDir;

#[derive(Debug, Default, cucumber::World)]
pub struct McpWorld {
    pub temp_dir: Option<TempDir>,
    /// A second tree the server root does *not* cover. Nothing here may ever
    /// be read, written or listed by a tool — it is the "outside" that a
    /// symlink planted inside the root would have to reach.
    outside_dir: Option<TempDir>,
    pub last_result: Option<Value>,
    pub last_error: Option<String>,
    /// How the server process of the last tool call terminated.
    pub server_exit: Option<ExitStatus>,
}

impl McpWorld {
    #[expect(
        clippy::expect_used,
        reason = "BDD steps fail by panicking; harness = false puts them outside the test allowances"
    )]
    pub fn temp_path(&self) -> PathBuf {
        self.temp_dir
            .as_ref()
            .expect("no temp dir")
            .path()
            .to_path_buf()
    }

    pub fn resolve(&self, relative: &str) -> String {
        self.temp_path().join(relative).display().to_string()
    }

    /// The out-of-root tree, created on first use.
    #[expect(
        clippy::expect_used,
        reason = "BDD steps fail by panicking; harness = false puts them outside the test allowances"
    )]
    pub fn outside_path(&mut self) -> PathBuf {
        self.outside_dir
            .get_or_insert_with(|| TempDir::new().expect("create outside dir"))
            .path()
            .to_path_buf()
    }

    /// Ignore initialization and malformed lines, accepting only this tool call's id.
    pub fn accept_tool_response(&mut self, line: &str) -> bool {
        let Ok(response) = serde_json::from_str::<Value>(line) else {
            return false;
        };
        if response.get("id") != Some(&json!(2)) {
            return false;
        }
        if let Some(error) = response.get("error") {
            self.last_error = Some(error["message"].as_str().unwrap_or("unknown").to_owned());
        } else if let Some(result) = response.get("result") {
            self.record_tool_result(result);
        }
        true
    }

    fn record_tool_result(&mut self, result: &Value) {
        let text = result
            .get("content")
            .and_then(|content| content.get(0))
            .and_then(|content| content.get("text"))
            .and_then(Value::as_str)
            .unwrap_or("");
        if result.get("isError") == Some(&json!(true)) {
            self.last_error = Some(text.to_owned());
        } else {
            // Structured JSON and plain text results remain equally inspectable by steps.
            self.last_result = Some(serde_json::from_str(text).unwrap_or_else(|_| json!(text)));
        }
    }

    pub fn result_text(&self) -> String {
        match &self.last_result {
            Some(v) => serde_json::to_string(v).unwrap_or_default(),
            None => String::new(),
        }
    }
}
