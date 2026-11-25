//! Purpose: Exercise the public automation CLI through private pipes and isolated workspaces.

use std::{fs, path::Path};

use serde_json::{json, Value};
use tempfile::TempDir;

mod catalog;
mod clipboard;
mod files;
mod input;
mod lifecycle;
mod maintenance;
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
