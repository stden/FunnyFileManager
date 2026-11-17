//! Purpose: Finds and reads the repository's Rust and Python sources for the line-limit guards.

use std::fs;
use std::path::{Path, PathBuf};

/// Where the sources live, relative to the repository root.
pub const SOURCE_ROOTS: [&str; 7] = [
    "src",
    "tests",
    "benches",
    "examples",
    "mcp",
    "colorer-sys",
    "scripts",
];

/// Directories that hold no sources of ours.
pub const SKIPPED_DIRS: [&str; 3] = ["target", "target-win", "node_modules"];

pub fn collect_rs_files(dir: &Path, out: &mut Vec<PathBuf>) {
    collect_sources(dir, &["rs"], out);
}

#[expect(
    clippy::expect_used,
    reason = "test fixture: an unreadable source tree fails the calling test"
)]
fn collect_sources(dir: &Path, extensions: &[&str], out: &mut Vec<PathBuf>) {
    for entry in fs::read_dir(dir).expect("read source directory") {
        let path = entry.expect("directory entry").path();
        let skipped = path
            .file_name()
            .and_then(|name| name.to_str())
            .is_some_and(|name| name.starts_with('.') || SKIPPED_DIRS.contains(&name));
        if path.is_dir() {
            if !skipped {
                collect_sources(&path, extensions, out);
            }
        } else if path
            .extension()
            .and_then(|ext| ext.to_str())
            .is_some_and(|ext| extensions.contains(&ext))
        {
            out.push(path);
        }
    }
}

/// Every Rust and Python source in the repository, with its path from the root.
#[expect(
    clippy::expect_used,
    reason = "test fixture: an unreadable repository root fails the calling test"
)]
pub fn repository_sources() -> Vec<(String, usize)> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let mut files = Vec::new();
    for dir in SOURCE_ROOTS {
        collect_sources(&root.join(dir), &["rs", "py"], &mut files);
    }
    for entry in fs::read_dir(root).expect("read repository root") {
        let path = entry.expect("directory entry").path();
        if path.extension().is_some_and(|ext| ext == "py") {
            files.push(path);
        }
    }
    read_sources(files)
        .into_iter()
        .map(|(path, text)| {
            let relative = path.strip_prefix(root).unwrap_or(&path);
            (
                relative.to_string_lossy().replace('\\', "/"),
                text.lines().count(),
            )
        })
        .collect()
}

#[expect(
    clippy::panic,
    reason = "test assertion: an unreadable source must fail the guard, and the should_panic \
              tests below pin exactly that panic"
)]
pub fn read_sources(files: Vec<PathBuf>) -> Vec<(PathBuf, String)> {
    files
        .into_iter()
        .map(|path| {
            let text = fs::read_to_string(&path)
                .unwrap_or_else(|error| panic!("read Rust source {}: {error}", path.display()));
            (path, text)
        })
        .collect()
}
