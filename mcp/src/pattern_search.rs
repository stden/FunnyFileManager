//! Glob and metadata-filtered searches, without following directory symlinks.

use super::{globmatch, json_response, walkable_type, FfmServer, GlobParams};
use globmatch::CompiledGlob;
use rmcp::{model::CallToolResult, ErrorData as McpError};
use serde_json::json;
use std::{fs, path::Path};

mod filters;
#[cfg(test)]
mod tests;
pub use filters::find;

pub fn glob(server: &FfmServer, params: &GlobParams) -> Result<CallToolResult, McpError> {
    let root = server.safe_path(&params.path)?;
    if !root.is_dir() {
        return Err(McpError::invalid_params(
            format!("not a directory: {}", root.display()),
            None,
        ));
    }
    let compiled =
        CompiledGlob::compile(&params.pattern).map_err(|e| McpError::invalid_params(e, None))?;

    let mut scan = GlobScan {
        pattern: &compiled,
        max_depth: compiled.max_depth(),
        max_results: params.max_results,
        rel_buf: String::new(),
        results: Vec::new(),
        visited: 0,
    };
    glob_recursive(&root, 0, &mut scan);

    let result = json!({
        "pattern": params.pattern,
        "root": root.display().to_string(),
        "count": scan.results.len(),
        "results": scan.results,
    });
    Ok(json_response(&result))
}

struct GlobScan<'a> {
    pattern: &'a CompiledGlob<'a>,
    /// `None` once `**` makes the depth unbounded.
    max_depth: Option<usize>,
    max_results: usize,
    rel_buf: String,
    results: Vec<serde_json::Value>,
    visited: usize,
}

fn glob_recursive(dir: &Path, depth: usize, scan: &mut GlobScan<'_>) {
    if scan.results.len() >= scan.max_results || scan.visited >= crate::walk::WALK_BUDGET {
        return;
    }
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        if scan.results.len() >= scan.max_results || scan.visited >= crate::walk::WALK_BUDGET {
            return;
        }
        scan.visited += 1;
        let Some(file_type) = walkable_type(&entry) else {
            continue;
        };
        visit_glob_entry(&entry, file_type.is_dir(), depth, scan);
    }
}

fn visit_glob_entry(entry: &fs::DirEntry, is_dir: bool, depth: usize, scan: &mut GlobScan<'_>) {
    let raw_name = entry.file_name();
    let name = raw_name.to_string_lossy();
    let base_len = scan.rel_buf.len();
    if base_len > 0 {
        scan.rel_buf.push('/');
    }
    scan.rel_buf.push_str(&name);

    let matched = scan.pattern.matches(&scan.rel_buf);
    let descend = is_dir && scan.max_depth.is_none_or(|max| depth < max);

    if matched || descend {
        let path = entry.path();
        if matched {
            let meta = entry.metadata().ok();
            scan.results.push(json!({
                "path": path.display().to_string(),
                "name": name.as_ref(),
                "relative_path": scan.rel_buf.as_str(),
                "type": if is_dir { "dir" } else { "file" },
                "size": meta.as_ref().map_or(0, fs::Metadata::len),
            }));
        }
        if descend {
            glob_recursive(&path, depth + 1, scan);
        }
    }
    scan.rel_buf.truncate(base_len);
}
