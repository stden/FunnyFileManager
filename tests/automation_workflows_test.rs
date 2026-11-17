//! Purpose: Exercise the public automation CLI through private pipes and isolated workspaces.
#![expect(
    clippy::unwrap_used,
    clippy::indexing_slicing,
    reason = "integration-test helpers: panics on fixture failure are correct behavior"
)]

use std::{fs, path::Path};

use serde_json::{json, Value};
use tempfile::TempDir;

#[path = "automation_workflows_test/catalog.rs"]
mod catalog;
#[path = "automation_workflows_test/clipboard.rs"]
mod clipboard;
#[path = "automation_workflows_test/files.rs"]
mod files;
#[path = "automation_workflows_test/input.rs"]
mod input;
#[path = "automation_workflows_test/lifecycle.rs"]
mod lifecycle;
#[path = "automation_workflows_test/session.rs"]
mod session;

use session::Session;

fn workspace() -> TempDir {
    let root = TempDir::new().unwrap();
    fs::create_dir(root.path().join("left")).unwrap();
    fs::create_dir(root.path().join("right")).unwrap();
    fs::write(root.path().join("left/file.txt"), "alpha\n").unwrap();
    root
}

fn assert_path(actual: &Value, expected: &Path) {
    let actual = fs::canonicalize(actual.as_str().unwrap()).unwrap();
    assert_eq!(actual, fs::canonicalize(expected).unwrap());
}

fn screen(state: &Value) -> String {
    state["screen"]["rows"]
        .as_array()
        .unwrap()
        .iter()
        .map(|row| row.as_str().unwrap())
        .collect::<Vec<_>>()
        .join("\n")
}
