//! Pattern deletion with post-order directory handling and dry-run reports.

use super::{
    format_size, globmatch::CompiledGlob, json_response, walkable_type, DeleteByPatternParams,
    FfmServer, WALK_BUDGET,
};
use rmcp::{model::CallToolResult, ErrorData as McpError};
use serde_json::json;
use std::{fs, path::Path};
use types::{DeleteKind, DeleteRun, SubtreeSizes};

#[cfg(test)]
mod tests;
mod types;

pub fn delete_by_pattern(
    server: &FfmServer,
    params: &DeleteByPatternParams,
) -> Result<CallToolResult, McpError> {
    let root = server.safe_path(&params.path)?;
    if !root.is_dir() {
        return Err(McpError::invalid_params(
            format!("not a directory: {}", root.display()),
            None,
        ));
    }
    let compiled =
        CompiledGlob::compile(&params.pattern).map_err(|e| McpError::invalid_params(e, None))?;

    let mut run = DeleteRun::default();
    delete_pattern_recursive(&root, &compiled, params, &mut run);

    let total_size: u64 = run.deleted.iter().filter_map(|d| d["size"].as_u64()).sum();
    let result = json!({
        "root": root.display().to_string(),
        "pattern": params.pattern,
        "dry_run": params.dry_run,
        "deleted_count": run.deleted.len(),
        "total_size": total_size,
        "total_size_human": format_size(total_size),
        "errors": run.errors.len(),
        "deleted": run.deleted,
        "error_details": run.errors,
    });
    Ok(json_response(&result))
}

fn remove_entry(kind: DeleteKind, path: &Path, recursive: bool) -> std::io::Result<()> {
    match kind {
        DeleteKind::Directory(_) if recursive => fs::remove_dir_all(path),
        DeleteKind::Directory(_) => fs::remove_dir(path),
        DeleteKind::File(_) => fs::remove_file(path),
    }
}

fn delete_pattern_recursive(
    dir: &Path,
    pattern: &CompiledGlob<'_>,
    params: &DeleteByPatternParams,
    run: &mut DeleteRun,
) -> SubtreeSizes {
    if run.visited >= WALK_BUDGET {
        return SubtreeSizes::default();
    }
    let Ok(entries) = fs::read_dir(dir) else {
        return SubtreeSizes::default();
    };
    let mut totals = SubtreeSizes::untouched(0);
    for entry in entries.flatten() {
        if run.visited >= WALK_BUDGET {
            totals.complete = false;
            break;
        }
        let child = delete_pattern_entry(&entry, pattern, params, run);
        totals.remaining += child.remaining;
        totals.claimed += child.claimed;
        totals.complete &= child.complete;
    }
    totals
}

fn delete_pattern_entry(
    entry: &fs::DirEntry,
    pattern: &CompiledGlob<'_>,
    params: &DeleteByPatternParams,
    run: &mut DeleteRun,
) -> SubtreeSizes {
    // Links remain excluded before size calculation or recursive deletion.
    let Some(file_type) = walkable_type(entry) else {
        return SubtreeSizes::untouched(entry.metadata().map_or(0, |meta| meta.len()));
    };
    run.visited += 1;
    let matched = pattern.matches(&entry.file_name().to_string_lossy());
    if file_type.is_dir() {
        return delete_dir_entry(entry, matched, pattern, params, run);
    }
    let bytes = entry.metadata().map_or(0, |meta| meta.len());
    if !matched {
        return SubtreeSizes::untouched(bytes);
    }
    let (claimed, removed) = delete_match(&entry.path(), DeleteKind::File(bytes), 0, params, run);
    SubtreeSizes {
        remaining: if removed { 0 } else { bytes },
        claimed,
        complete: true,
    }
}

fn delete_dir_entry(
    entry: &fs::DirEntry,
    matched: bool,
    pattern: &CompiledGlob<'_>,
    params: &DeleteByPatternParams,
    run: &mut DeleteRun,
) -> SubtreeSizes {
    if !(params.recursive || params.include_dirs && matched) {
        return SubtreeSizes::default();
    }
    let path = entry.path();
    let mut sub = if params.recursive {
        delete_pattern_recursive(&path, pattern, params, run)
    } else {
        SubtreeSizes::default()
    };
    if params.include_dirs && matched {
        let known = sub.complete.then_some(sub.remaining);
        let (added, removed) = delete_match(
            &path,
            DeleteKind::Directory(known),
            sub.claimed,
            params,
            run,
        );
        sub.claimed += added;
        if removed {
            sub.remaining = 0;
        }
    }
    sub
}

fn delete_match(
    path: &Path,
    kind: DeleteKind,
    child_claimed: u64,
    params: &DeleteByPatternParams,
    run: &mut DeleteRun,
) -> (u64, bool) {
    let raw = kind.size(path);
    let size = if params.dry_run {
        raw.saturating_sub(child_claimed)
    } else {
        raw
    };
    if params.dry_run {
        run.deleted.push(json!({
            "path": path.display().to_string(),
            "type": kind.label(),
            "size": size,
            "would_delete": true,
        }));
        return (size, false);
    }
    if let Err(error) = remove_entry(kind, path, params.recursive) {
        run.errors.push(json!({
            "path": path.display().to_string(),
            "error": error.to_string(),
        }));
        return (0, false);
    }
    run.deleted.push(json!({
        "path": path.display().to_string(),
        "type": kind.label(),
        "size": size,
    }));
    (size, true)
}
