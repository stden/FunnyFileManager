//! Glob and metadata-filtered searches, without following directory symlinks.

use super::{
    format_size, globmatch, relative_slash_path, walkable_type, FfmServer, FindParams, GlobParams,
};
use rmcp::{
    model::{CallToolResult, ContentBlock},
    ErrorData as McpError,
};
use serde_json::json;
use std::{fs, path::Path};

pub(super) fn glob(server: &FfmServer, params: GlobParams) -> Result<CallToolResult, McpError> {
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
    Ok(CallToolResult::success(vec![ContentBlock::text(
        serde_json::to_string_pretty(&result).unwrap_or_default(),
    )]))
}

pub(super) fn find(server: &FfmServer, params: FindParams) -> Result<CallToolResult, McpError> {
    let root = server.safe_path(&params.path)?;
    if !root.is_dir() {
        return Err(McpError::invalid_params(
            format!("not a directory: {}", root.display()),
            None,
        ));
    }

    let now = std::time::SystemTime::now();
    let mut results = Vec::new();

    find_recursive(&root, 0, &params, &now, &mut results);

    let result = json!({
        "root": root.display().to_string(),
        "count": results.len(),
        "results": results,
    });
    Ok(CallToolResult::success(vec![ContentBlock::text(
        serde_json::to_string_pretty(&result).unwrap_or_default(),
    )]))
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
    let entries = match fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return,
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

fn find_recursive(
    dir: &Path,
    depth: usize,
    params: &FindParams,
    now: &std::time::SystemTime,
    results: &mut Vec<serde_json::Value>,
) {
    if results.len() >= params.max_results {
        return;
    }
    if params.max_depth.is_some_and(|max| depth > max) {
        return;
    }

    let entries = match fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return,
    };

    for entry in entries.flatten() {
        if results.len() >= params.max_results {
            return;
        }

        visit_find_entry(&entry, depth, params, now, results);
    }
}

/// Filter metadata without following links; missing or future timestamps remain eligible.
fn matches_filters(
    name: &str,
    kind: &str,
    meta: &fs::Metadata,
    params: &FindParams,
    now: &std::time::SystemTime,
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
            "size": meta.as_ref().map(|m| m.len()).unwrap_or(0),
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

fn visit_find_entry(
    entry: &fs::DirEntry,
    depth: usize,
    params: &FindParams,
    now: &std::time::SystemTime,
    results: &mut Vec<serde_json::Value>,
) {
    let name = entry.file_name().to_string_lossy().to_string();
    let meta = match fs::symlink_metadata(entry.path()) {
        Ok(m) => m,
        Err(_) => return,
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
        find_recursive(&entry.path(), depth + 1, params, now, results);
    }
}
