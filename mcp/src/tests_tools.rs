//! In-process tests for the rules that must hold at *every* call site: the
//! registered tool list, and the promise that a symlink is never silently
//! stood in for the data it points at.
//!
//! These live beside `tests.rs` rather than inside it because they are
//! cross-cutting: each one has a twin somewhere else among the tools, and
//! keeping them together is what makes a missing twin obvious. The helpers
//! live here; the tests, one child module per behaviour.

use super::*;
use rmcp::handler::server::wrapper::Parameters;
use rmcp::model::CallToolResult;
use std::fmt::Write as _;
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

/// `line 1` through `line {last}`, one per line, newline-terminated.
fn numbered_lines(last: u32) -> String {
    (1..=last).fold(String::new(), |mut body, n| {
        writeln!(body, "line {n}").expect("writing into a `String` cannot fail");
        body
    })
}

fn write_file_at(path: &Path, content: &str) {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).expect("create parent directory");
    }
    fs::write(path, content).expect("write fixture file");
}

/// Assert that `path` is still a symlink.
///
/// `symlink_metadata`, never `Path::exists()`: `exists()` follows the link, so
/// it passes just as happily when the link has been replaced by a regular file
/// — or fails on a link that is merely dangling. Both make the assertion a lie.
#[cfg(unix)]
fn assert_is_symlink(path: &Path) {
    let meta = fs::symlink_metadata(path)
        .unwrap_or_else(|e| panic!("symlink_metadata {}: {e}", path.display()));
    assert!(
        meta.file_type().is_symlink(),
        "{} should still be a symlink",
        path.display()
    );
}

#[cfg(unix)]
fn assert_gone(path: &Path) {
    assert!(
        fs::symlink_metadata(path).is_err(),
        "{} should have been removed",
        path.display()
    );
}

/// A secret file in a tree the server root does not cover, plus a link to it
/// planted *inside* the root — the shape every "walk leaves the root" test
/// needs. Returns the outside directory, which must outlive the test.
#[cfg(unix)]
fn plant_link_out_of_root(root: &Path, link_name: &str, secret: &str) -> TempDir {
    let outside = TempDir::new().expect("temp dir");
    let target = outside.path().join("secret.txt");
    write_file_at(&target, secret);
    std::os::unix::fs::symlink(&target, root.join(link_name)).expect("plant the link");
    outside
}

mod tests_analysis;
mod tests_compare;
mod tests_duplicates;
mod tests_file_names;
mod tests_largest;
#[cfg(unix)]
mod tests_links;
mod tests_reading;
mod tests_recent;
mod tests_router;
mod tests_tail;
mod tests_text;
#[cfg(unix)]
mod tests_walkers;
mod tests_which;
