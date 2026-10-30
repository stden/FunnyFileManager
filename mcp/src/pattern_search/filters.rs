//! Purpose: `find`: a walk that reports entries passing its name, type, size,
//! depth and age filters, never descending through a link.

use crate::conversions::format_size;
use crate::tools::{json_response, patterns::FindParams};
use crate::{globmatch, FfmServer};
use rmcp::{model::CallToolResult, ErrorData as McpError};
use serde_json::json;
use std::{fs, path::Path};

pub fn find(server: &FfmServer, params: &FindParams) -> Result<CallToolResult, McpError> {
    let root = server.safe_path(&params.path)?;
    if !root.is_dir() {
        return Err(McpError::invalid_params(
            format!("not a directory: {}", root.display()),
            None,
        ));
    }
    if let Some(pattern) = &params.name {
        globmatch::validate_pattern(pattern).map_err(|e| McpError::invalid_params(e, None))?;
    }

    let now = std::time::SystemTime::now();
    let mut results = Vec::new();
    let mut visited = 0usize;

    find_recursive(&root, 0, params, now, &mut results, &mut visited);

    let result = json!({
        "root": root.display().to_string(),
        "count": results.len(),
        "results": results,
    });
    Ok(json_response(&result))
}

fn find_recursive(
    dir: &Path,
    depth: usize,
    params: &FindParams,
    now: std::time::SystemTime,
    results: &mut Vec<serde_json::Value>,
    visited: &mut usize,
) {
    if results.len() >= params.max_results || *visited >= crate::walk::WALK_BUDGET {
        return;
    }
    if params.max_depth.is_some_and(|max| depth > max) {
        return;
    }

    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };

    for entry in entries.flatten() {
        if results.len() >= params.max_results || *visited >= crate::walk::WALK_BUDGET {
            return;
        }
        *visited += 1;

        visit_find_entry(&entry, depth, params, now, results, visited);
    }
}

/// Filter metadata without following links; missing or future timestamps remain eligible.
fn matches_filters(
    name: &str,
    kind: &str,
    meta: &fs::Metadata,
    params: &FindParams,
    now: std::time::SystemTime,
) -> bool {
    if params
        .name
        .as_deref()
        .is_some_and(|pattern| !globmatch::matches(name, pattern))
    {
        return false;
    }
    if params
        .file_type
        .as_deref()
        .is_some_and(|wanted| wanted != kind)
    {
        return false;
    }
    if params.min_size.is_some_and(|min| meta.len() < min)
        || params.max_size.is_some_and(|max| meta.len() > max)
    {
        return false;
    }
    let Some(secs) = params.modified_within_secs else {
        return true;
    };
    let Ok(modified) = meta.modified() else {
        return true;
    };
    let Ok(elapsed) = now.duration_since(modified) else {
        return true;
    };
    elapsed.as_secs() <= secs
}

fn visit_find_entry(
    entry: &fs::DirEntry,
    depth: usize,
    params: &FindParams,
    now: std::time::SystemTime,
    results: &mut Vec<serde_json::Value>,
    visited: &mut usize,
) {
    let name = entry.file_name().to_string_lossy().to_string();
    let Ok(meta) = fs::symlink_metadata(entry.path()) else {
        return;
    };

    let is_dir = meta.is_dir();
    let is_symlink = meta.is_symlink();
    let file_type_str = if is_symlink {
        "symlink"
    } else if is_dir {
        "dir"
    } else {
        "file"
    };

    if matches_filters(&name, file_type_str, &meta, params, now) {
        results.push(json!({
            "path": entry.path().display().to_string(),
            "name": name,
            "type": file_type_str,
            "size": meta.len(),
            "size_human": format_size(meta.len()),
        }));
    }

    if is_dir && !is_symlink {
        find_recursive(&entry.path(), depth + 1, params, now, results, visited);
    }
}
