//! Purpose: `find`: a walk that reports entries passing its name, type, size,
//! depth and age filters, never descending through a link.

use crate::conversions::format_size;
use crate::globmatch::CompiledGlob;
use crate::tools::{json_response, patterns::FindParams};
use crate::FfmServer;
use rmcp::{model::CallToolResult, ErrorData as McpError};
use serde_json::json;
use std::{fs, path::Path, time::SystemTime};

#[cfg(test)]
mod tests;

pub fn find(server: &FfmServer, params: &FindParams) -> Result<CallToolResult, McpError> {
    let root = server.safe_path(&params.path)?;
    if !root.is_dir() {
        return Err(McpError::invalid_params(
            format!("not a directory: {}", root.display()),
            None,
        ));
    }
    let compiled = params
        .name
        .as_deref()
        .map(CompiledGlob::compile)
        .transpose()
        .map_err(|e| McpError::invalid_params(e, None))?;

    let mut run = FindRun {
        params,
        name_glob: compiled.as_ref(),
        now: SystemTime::now(),
        results: Vec::new(),
        visited: 0,
    };
    find_recursive(&root, 0, &mut run);

    let result = json!({
        "root": root.display().to_string(),
        "count": run.results.len(),
        "results": run.results,
    });
    Ok(json_response(&result))
}

struct FindRun<'a> {
    params: &'a FindParams,
    name_glob: Option<&'a CompiledGlob<'a>>,
    now: SystemTime,
    results: Vec<serde_json::Value>,
    visited: usize,
}

fn find_recursive(dir: &Path, depth: usize, run: &mut FindRun<'_>) {
    if run.results.len() >= run.params.max_results || run.visited >= crate::walk::WALK_BUDGET {
        return;
    }
    if run.params.max_depth.is_some_and(|max| depth > max) {
        return;
    }
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        if run.results.len() >= run.params.max_results || run.visited >= crate::walk::WALK_BUDGET {
            return;
        }
        run.visited += 1;
        visit_find_entry(&entry, depth, run);
    }
}

fn matches_metadata(meta: &fs::Metadata, params: &FindParams, now: SystemTime) -> bool {
    if params.min_size.is_some_and(|min| meta.len() < min)
        || params.max_size.is_some_and(|max| meta.len() > max)
    {
        return false;
    }
    let Some(secs) = params.modified_within_secs else {
        return true;
    };
    modified_within(&meta.modified(), now, secs)
}

/// Missing or future timestamps cannot prove that an entry is too old.
fn modified_within(modified: &std::io::Result<SystemTime>, now: SystemTime, secs: u64) -> bool {
    let Ok(modified) = modified else {
        return true;
    };
    let Ok(elapsed) = now.duration_since(*modified) else {
        return true;
    };
    elapsed.as_secs() <= secs
}

fn visit_find_entry(entry: &fs::DirEntry, depth: usize, run: &mut FindRun<'_>) {
    let (is_symlink, is_dir) = entry.file_type().map_or((false, false), |ft| {
        (ft.is_symlink(), ft.is_dir() && !ft.is_symlink())
    });
    let kind = entry_kind(is_symlink, is_dir);
    let raw_name = entry.file_name();
    let name = raw_name.to_string_lossy();
    let eligible = run.name_glob.is_none_or(|glob| glob.matches(&name))
        && run
            .params
            .file_type
            .as_deref()
            .is_none_or(|wanted| wanted == kind);
    if eligible || is_dir {
        let path = entry.path();
        if eligible {
            record_find_match(&path, &name, kind, run);
        }
        if is_dir {
            find_recursive(&path, depth + 1, run);
        }
    }
}

const fn entry_kind(is_symlink: bool, is_dir: bool) -> &'static str {
    if is_symlink {
        "symlink"
    } else if is_dir {
        "dir"
    } else {
        "file"
    }
}

fn record_find_match(path: &Path, name: &str, kind: &str, run: &mut FindRun<'_>) {
    if let Ok(meta) = fs::symlink_metadata(path) {
        if matches_metadata(&meta, run.params, run.now) {
            run.results.push(json!({
                "path": path.display().to_string(),
                "name": name,
                "type": kind,
                "size": meta.len(),
                "size_human": format_size(meta.len()),
            }));
        }
    }
}
