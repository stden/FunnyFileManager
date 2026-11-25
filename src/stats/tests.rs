//! Unit tests: measuring a directory, and the limits that keep it safe.

use super::*;
use std::fs;
use tempfile::TempDir;

mod budget;
mod coverage;
mod kinds;
mod largest;
mod measure;
mod safety;
mod totals;

/// A tree built from `(relative path, size in bytes)`.
fn tree(files: &[(&str, u64)]) -> TempDir {
    let dir = TempDir::new().expect("temp dir");
    for (path, size) in files {
        let full = dir.path().join(path);
        if let Some(parent) = full.parent() {
            fs::create_dir_all(parent).expect("parents");
        }
        let size = usize::try_from(*size).expect("fixture size fits usize");
        fs::write(&full, vec![b'x'; size]).expect("file");
    }
    dir
}

fn child(stats: &DirectoryStats, name: &str) -> Option<ChildSize> {
    stats.children.iter().find(|c| c.name == name).cloned()
}

fn kind(stats: &DirectoryStats, kind: ContentKind) -> Option<KindTotal> {
    stats.by_kind.iter().copied().find(|k| k.kind == kind)
}
