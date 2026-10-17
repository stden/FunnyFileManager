//! Pattern deletion with post-order directory handling and dry-run reports.

use super::{
    format_size, globmatch, json_response, walkable_type, DeleteByPatternParams, FfmServer,
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

fn delete_pattern_recursive(dir: &Path, params: &DeleteByPatternParams, run: &mut DeleteRun) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        // Links remain excluded before size calculation or recursive deletion.
        let Some(file_type) = walkable_type(&entry) else {
            continue;
        };
        let path = entry.path();
        let name = entry.file_name().to_string_lossy().to_string();
        if file_type.is_dir() {
            if params.recursive {
                delete_pattern_recursive(&path, params, run);
            }
            if params.include_dirs && globmatch::matches(&name, &params.pattern) {
                delete_match(&path, DeleteKind::Directory, params, run);
            }
        } else if globmatch::matches(&name, &params.pattern) {
            delete_match(&path, DeleteKind::File, params, run);
        }
    }
}

/// Report the entry at the same point in the post-order traversal for preview or deletion.
fn delete_match(
    path: &Path,
    kind: DeleteKind,
    params: &DeleteByPatternParams,
    run: &mut DeleteRun,
) {
    let size = kind.size(path);
    if params.dry_run {
        run.deleted.push(json!({
            "path": path.display().to_string(),
            "type": kind.label(),
            "size": size,
            "would_delete": true,
        }));
        return;
    }
    match kind.remove(path, params.recursive) {
        Ok(()) => run.deleted.push(json!({
            "path": path.display().to_string(),
            "type": kind.label(),
            "size": size,
        })),
        Err(error) => run.errors.push(json!({
            "path": path.display().to_string(),
            "error": error.to_string(),
        })),
    }
}
