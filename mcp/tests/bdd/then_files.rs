//! Purpose: `Then` steps that check the filesystem a tool call left behind.

use crate::world::McpWorld;
use cucumber::then;
use std::fs;

#[then(expr = "the file {string} should exist")]
async fn then_file_exists(w: &mut McpWorld, name: String) {
    let path = w.temp_path().join(&name);
    assert!(path.exists(), "file should exist: {}", path.display());
}

#[then(expr = "the file {string} should not exist")]
async fn then_file_not_exists(w: &mut McpWorld, name: String) {
    let path = w.temp_path().join(&name);
    assert!(!path.exists(), "file should not exist: {}", path.display());
}

#[then(expr = "the file {string} should contain {string}")]
#[expect(
    clippy::panic,
    reason = "BDD steps fail by panicking; harness = false puts them outside the test allowances"
)]
async fn then_file_contains(w: &mut McpWorld, name: String, expected: String) {
    let path = w.temp_path().join(&name);
    let content =
        fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {}: {}", path.display(), e));
    assert!(
        content.contains(&expected),
        "file '{name}' should contain '{expected}', got '{content}'"
    );
}

#[then(expr = "the file {string} should not contain {string}")]
#[expect(
    clippy::panic,
    reason = "BDD steps fail by panicking; harness = false puts them outside the test allowances"
)]
async fn then_file_not_contains(w: &mut McpWorld, name: String, unexpected: String) {
    let path = w.temp_path().join(&name);
    let content =
        fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {}: {}", path.display(), e));
    assert!(
        !content.contains(&unexpected),
        "file '{name}' should NOT contain '{unexpected}', got '{content}'"
    );
}

#[then(expr = "the symlink {string} should not exist")]
async fn then_symlink_not_exists(w: &mut McpWorld, name: String) {
    // `exists()` follows the link, so it cannot tell a deleted link from a
    // dangling one — ask for the link's own metadata instead.
    let path = w.temp_path().join(&name);
    assert!(
        fs::symlink_metadata(&path).is_err(),
        "symlink should be gone: {}",
        path.display()
    );
}

#[then(expr = "{string} should still be a symlink")]
#[expect(
    clippy::panic,
    reason = "BDD steps fail by panicking; harness = false puts them outside the test allowances"
)]
async fn then_still_a_symlink(w: &mut McpWorld, name: String) {
    // `exists()` follows the link and would pass even after the link had been
    // renamed away and replaced by a regular file — ask for its own metadata.
    let path = w.temp_path().join(&name);
    let meta = fs::symlink_metadata(&path)
        .unwrap_or_else(|e| panic!("symlink_metadata {}: {}", path.display(), e));
    assert!(
        meta.file_type().is_symlink(),
        "{} should still be a symlink",
        path.display()
    );
}

#[then(expr = "the directory {string} should exist")]
async fn then_dir_exists(w: &mut McpWorld, name: String) {
    let path = w.temp_path().join(&name);
    assert!(path.is_dir(), "directory should exist: {}", path.display());
}
