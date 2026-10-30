//! Pattern deletion with post-order directory handling and dry-run reports.

use super::{
    format_size, globmatch, json_response, walkable_type, DeleteByPatternParams, FfmServer,
    WALK_BUDGET,
};
use ffm_mcp::tree_walk::directory_size;
use rmcp::{model::CallToolResult, ErrorData as McpError};
use serde_json::json;
use std::{fs, path::Path};

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
    globmatch::validate_pattern(&params.pattern).map_err(|e| McpError::invalid_params(e, None))?;

    let mut run = DeleteRun::default();

    delete_pattern_recursive(&root, params, &mut run);

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

#[derive(Default)]
struct DeleteRun {
    deleted: Vec<serde_json::Value>,
    errors: Vec<serde_json::Value>,
    visited: usize,
}

#[derive(Clone, Copy)]
enum DeleteKind {
    Directory,
    File,
}

impl DeleteKind {
    fn label(self) -> &'static str {
        match self {
            Self::Directory => "dir",
            Self::File => "file",
        }
    }

    fn size(self, path: &Path) -> u64 {
        match self {
            Self::Directory => directory_size(path),
            Self::File => fs::metadata(path).map_or(0, |meta| meta.len()),
        }
    }

    fn remove(self, path: &Path, recursive: bool) -> std::io::Result<()> {
        match self {
            Self::Directory => {
                if recursive {
                    fs::remove_dir_all(path)
                } else {
                    fs::remove_dir(path)
                }
            }
            Self::File => fs::remove_file(path),
        }
    }
}

fn delete_pattern_recursive(
    dir: &Path,
    params: &DeleteByPatternParams,
    run: &mut DeleteRun,
) -> u64 {
    if run.visited >= WALK_BUDGET {
        return 0;
    }
    let Ok(entries) = fs::read_dir(dir) else {
        return 0;
    };
    let mut claimed = 0;
    for entry in entries.flatten() {
        if run.visited >= WALK_BUDGET {
            break;
        }
        claimed += delete_pattern_entry(&entry, params, run);
    }
    claimed
}

fn delete_pattern_entry(
    entry: &fs::DirEntry,
    params: &DeleteByPatternParams,
    run: &mut DeleteRun,
) -> u64 {
    // Links remain excluded before size calculation or recursive deletion.
    let Some(file_type) = walkable_type(entry) else {
        return 0;
    };
    run.visited += 1;
    let path = entry.path();
    let matched = globmatch::matches(&entry.file_name().to_string_lossy(), &params.pattern);
    if file_type.is_dir() {
        let mut claimed = if params.recursive {
            delete_pattern_recursive(&path, params, run)
        } else {
            0
        };
        if params.include_dirs && matched {
            claimed += delete_match(&path, DeleteKind::Directory, claimed, params, run);
        }
        claimed
    } else if matched {
        delete_match(&path, DeleteKind::File, 0, params, run)
    } else {
        0
    }
}

/// Report the entry at the same point in the post-order traversal for preview or deletion.
fn delete_match(
    path: &Path,
    kind: DeleteKind,
    child_claimed: u64,
    params: &DeleteByPatternParams,
    run: &mut DeleteRun,
) -> u64 {
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
        return size;
    }
    if let Err(error) = kind.remove(path, params.recursive) {
        run.errors.push(json!({
            "path": path.display().to_string(),
            "error": error.to_string(),
        }));
        return 0;
    }
    run.deleted.push(json!({
        "path": path.display().to_string(),
        "type": kind.label(),
        "size": size,
    }));
    size
}
