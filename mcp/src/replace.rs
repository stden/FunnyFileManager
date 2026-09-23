//! Filtered text replacement with dry-run and per-file outcome reporting.

use super::{fsops, globmatch, json_response, walkable_type, FfmServer, ReplaceParams};
use rmcp::{model::CallToolResult, ErrorData as McpError};
use serde_json::json;
use std::{fs, path::Path};

pub fn replace(server: &FfmServer, params: &ReplaceParams) -> Result<CallToolResult, McpError> {
    let root = server.safe_path(&params.path)?;
    if !root.is_dir() {
        return Err(McpError::invalid_params(
            format!("not a directory: {}", root.display()),
            None,
        ));
    }

    if params.find.is_empty() {
        return Err(McpError::invalid_params(
            "find pattern must not be empty",
            None,
        ));
    }

    let opts = ReplaceOpts {
        find: &params.find,
        replace_with: &params.replace,
        ext_filter: params.extension.as_deref(),
        name_pattern: params.name_pattern.as_deref(),
        dry_run: params.dry_run,
    };
    let mut run = ReplaceRun::default();
    replace_recursive(&root, &opts, &mut run);

    Ok(replacement_report(&root, params, &run))
}

struct ReplaceOpts<'a> {
    find: &'a str,
    replace_with: &'a str,
    ext_filter: Option<&'a str>,
    name_pattern: Option<&'a str>,
    dry_run: bool,
}

#[derive(Default)]
struct ReplaceRun {
    changed: Vec<serde_json::Value>,
    errors: Vec<serde_json::Value>,
    total: usize,
}

fn replace_recursive(dir: &Path, opts: &ReplaceOpts<'_>, run: &mut ReplaceRun) {
    let entries = match fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return,
    };
    for entry in entries.flatten() {
        // A link is never opened: `read_to_string` below would follow it
        // out of the server root, and the rewrite would then copy that
        // outside content back in (see `walkable_type`).
        let Some(file_type) = walkable_type(&entry) else {
            continue;
        };
        let path = entry.path();

        if file_type.is_dir() {
            replace_recursive(&path, opts, run);
            continue;
        }

        let name = entry.file_name().to_string_lossy().to_string();
        replace_file(&path, &name, opts, run);
    }
}

/// Replace one readable text file, recording success only after a completed write.
fn replace_file(path: &Path, name: &str, opts: &ReplaceOpts<'_>, run: &mut ReplaceRun) {
    if !matches_file(name, opts) {
        return;
    }

    // Keep the original byte cap and ignore unreadable or non-UTF-8 files.
    let size = fs::metadata(path).map(|m| m.len()).unwrap_or(0);
    if size > 10 * 1024 * 1024 {
        return;
    }

    let Ok(content) = fs::read_to_string(path) else {
        return;
    };

    let count = content.matches(opts.find).count();
    if count == 0 {
        return;
    }

    if opts.dry_run {
        record_replacement(path, count, run);
        return;
    }

    let new_content = content.replace(opts.find, opts.replace_with);
    match fsops::write_replaced(path, &new_content) {
        Ok(()) => record_replacement(path, count, run),
        Err(e) => run.errors.push(json!({
            "path": path.display().to_string(),
            "error": e.to_string(),
        })),
    }
}

fn record_replacement(path: &Path, count: usize, run: &mut ReplaceRun) {
    run.total += count;
    run.changed.push(json!({
        "path": path.display().to_string(),
        "replacements": count,
    }));
}

fn replacement_report(root: &Path, params: &ReplaceParams, run: &ReplaceRun) -> CallToolResult {
    let result = json!({
        "root": root.display().to_string(),
        "find": params.find,
        "replace": params.replace,
        "dry_run": params.dry_run,
        "files_affected": run.changed.len(),
        "total_replacements": run.total,
        "files": run.changed,
        "errors": run.errors.len(),
        "error_details": run.errors,
    });
    json_response(&result)
}

fn matches_file(name: &str, opts: &ReplaceOpts<'_>) -> bool {
    if let Some(ext) = opts.ext_filter {
        let file_ext = Path::new(name)
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("");
        if !file_ext.eq_ignore_ascii_case(ext) {
            return false;
        }
    }

    if opts
        .name_pattern
        .is_some_and(|pattern| !globmatch::matches(name, pattern))
    {
        return false;
    }

    true
}
