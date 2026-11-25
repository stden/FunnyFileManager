//! Purpose: Lexical `.` / `..` normalization helpers for MCP path sanitization.

use std::path::Component;
#[cfg(test)]
use std::path::{Path, PathBuf};

/// Fold `.` and `..` away textually — for paths that do not exist yet.
#[cfg(test)]
pub fn normalize_lexically(path: &Path) -> PathBuf {
    let mut out = PathBuf::new();
    for component in path.components() {
        if is_cur_dir(&component) {
            continue;
        }
        if is_parent_dir(&component) {
            out.pop();
        } else {
            out.push(component);
        }
    }
    out
}

pub(super) fn is_cur_dir(component: &Component<'_>) -> bool {
    matches!(component, Component::CurDir)
        || matches!(component, Component::Normal(part) if *part == ".")
}

pub(super) fn is_parent_dir(component: &Component<'_>) -> bool {
    matches!(component, Component::ParentDir)
        || matches!(component, Component::Normal(part) if *part == "..")
}
