//! Purpose: A temporary project root for tests that start language servers.

use std::fs;

use tempfile::TempDir;

/// A temporary directory with its own `.git` marker. Language servers are
/// found by walking up from a file to a root marker, and `.git` is one of
/// them. Without the marker, a `TempDir` under a directory that is itself a
/// git repository (`/tmp` on some machines) resolves to that directory.
pub fn project_root() -> TempDir {
    let root = TempDir::new().expect("temporary project root");
    fs::create_dir(root.path().join(".git")).expect("project marker");
    root
}
