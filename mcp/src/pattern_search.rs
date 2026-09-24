//! Glob and metadata-filtered searches, without following directory symlinks.

use super::{globmatch, json_response, relative_slash_path, walkable_type, FfmServer, GlobParams};
use rmcp::{model::CallToolResult, ErrorData as McpError};
use serde_json::json;
use std::{fs, path::Path};

mod filters;
pub use filters::find;

pub fn glob(server: &FfmServer, params: &GlobParams) -> Result<CallToolResult, McpError> {
    let root = server.safe_path(&params.path)?;
    if !root.is_dir() {
        return Err(McpError::invalid_params(
            format!("not a directory: {}", root.display()),
            None,
        ));
    }

    let scan = GlobScan {
        root: &root,
        pattern: &params.pattern,
        max_depth: globmatch::max_depth(&params.pattern),
        max_results: params.max_results,
    };
    let mut results = Vec::new();
    glob_recursive(&root, 0, &scan, &mut results);

    let result = json!({
        "pattern": params.pattern,
        "root": root.display().to_string(),
        "count": results.len(),
        "results": results,
    });
    Ok(json_response(&result))
}

struct GlobScan<'a> {
    root: &'a Path,
    pattern: &'a str,
    /// `None` once `**` makes the depth unbounded.
    max_depth: Option<usize>,
    max_results: usize,
}

fn glob_recursive(
    dir: &Path,
    depth: usize,
    scan: &GlobScan<'_>,
    results: &mut Vec<serde_json::Value>,
) {
    if results.len() >= scan.max_results {
        return;
    }
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        if results.len() >= scan.max_results {
            return;
        }
        // A link is never reported: `entry.metadata()` below follows it,
        // so its size would describe a file outside the server root
        // (see `walkable_type`).
        let Some(file_type) = walkable_type(&entry) else {
            continue;
        };
        visit_glob_entry(&entry, file_type.is_dir(), depth, scan, results);
    }
}

fn visit_glob_entry(
    entry: &fs::DirEntry,
    is_dir: bool,
    depth: usize,
    scan: &GlobScan<'_>,
    results: &mut Vec<serde_json::Value>,
) {
    let path = entry.path();
    let relative = relative_slash_path(scan.root, &path);

    if globmatch::matches(&relative, scan.pattern) {
        let meta = entry.metadata().ok();
        results.push(json!({
            "path": path.display().to_string(),
            "name": entry.file_name().to_string_lossy(),
            "relative_path": relative,
            "type": if is_dir { "dir" } else { "file" },
            "size": meta.as_ref().map_or(0, fs::Metadata::len),
        }));
    }

    let descend = match scan.max_depth {
        Some(max) => depth < max,
        None => true,
    };
    if is_dir && descend {
        glob_recursive(&path, depth + 1, scan, results);
    }
}
