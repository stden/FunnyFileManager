//! Filtered text replacement with dry-run and per-file outcome reporting.

use super::{fsops, globmatch, json_response, walkable_type, FfmServer, ReplaceParams};
use globmatch::CompiledGlob;
use rmcp::{model::CallToolResult, ErrorData as McpError};
use serde_json::json;
use std::{fs, path::Path};

#[cfg(test)]
mod tests;

pub fn replace(server: &FfmServer, params: &ReplaceParams) -> Result<CallToolResult, McpError> {
    let root = server.safe_path(&params.path)?;
    let compiled_name = validate_replace(&root, params)?;
    let opts = ReplaceOpts {
        find: &params.find,
        replace_with: &params.replace,
        ext_filter: params.extension.as_deref(),
        name_pattern: compiled_name.as_ref(),
        dry_run: params.dry_run,
    };
    let mut run = ReplaceRun::default();
    replace_recursive(&root, &opts, &mut run);
    Ok(replacement_report(&root, params, &run))
}

fn validate_replace<'a>(
    root: &Path,
    params: &'a ReplaceParams,
) -> Result<Option<CompiledGlob<'a>>, McpError> {
    if !root.is_dir() {
        let msg = format!("not a directory: {}", root.display());
        return Err(McpError::invalid_params(msg, None));
    }
    if params.find.is_empty() {
        let msg = "find pattern must not be empty";
        return Err(McpError::invalid_params(msg, None));
    }
    params
        .name_pattern
        .as_deref()
        .map(CompiledGlob::compile)
        .transpose()
        .map_err(|e| McpError::invalid_params(e, None))
}

struct ReplaceOpts<'a> {
    find: &'a str,
    replace_with: &'a str,
    ext_filter: Option<&'a str>,
    name_pattern: Option<&'a CompiledGlob<'a>>,
    dry_run: bool,
}

const MAX_FILE_BYTES_USIZE: usize = 10 * 1024 * 1024;

#[derive(Default)]
struct ReplaceRun {
    changed: Vec<serde_json::Value>,
    errors: Vec<serde_json::Value>,
    total: usize,
    visited: usize,
}

fn replace_recursive(dir: &Path, opts: &ReplaceOpts<'_>, run: &mut ReplaceRun) {
    if run.visited >= crate::walk::WALK_BUDGET {
        return;
    }
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        if run.visited >= crate::walk::WALK_BUDGET {
            return;
        }
        run.visited += 1;
        // A link is never opened: `read_to_string` would follow it out of the root.
        let Some(file_type) = walkable_type(&entry) else {
            continue;
        };
        if file_type.is_dir() {
            replace_recursive(&entry.path(), opts, run);
            continue;
        }
        #[expect(
            clippy::filetype_is_file,
            reason = "skip FIFOs, sockets and device nodes"
        )]
        if file_type.is_file() {
            replace_file(&entry, opts, run);
        }
    }
}

/// Replace one readable text file, recording success only after a completed write.
fn replace_file(entry: &fs::DirEntry, opts: &ReplaceOpts<'_>, run: &mut ReplaceRun) {
    let raw_name = entry.file_name();
    let name = raw_name.to_string_lossy();
    if !matches_file(&name, opts) || entry.metadata().map_or(0, |m| m.len()) > 10 * 1024 * 1024 {
        return;
    }
    let path = entry.path();
    let Ok(content) = fs::read_to_string(&path) else {
        return;
    };
    let Some(first) = content.find(opts.find) else {
        return;
    };
    apply_replacement(&path, &content, first, opts, run);
}

fn apply_replacement(
    path: &Path,
    content: &str,
    first: usize,
    opts: &ReplaceOpts<'_>,
    run: &mut ReplaceRun,
) {
    let rest = content.get(first + opts.find.len()..).unwrap_or_default();
    let count = 1 + rest.matches(opts.find).count();
    let projected = content
        .len()
        .saturating_sub(count.saturating_mul(opts.find.len()))
        .saturating_add(count.saturating_mul(opts.replace_with.len()));
    if projected > MAX_FILE_BYTES_USIZE {
        let msg = "replacement output exceeds maximum file size (10 MiB)";
        return record_error(path, msg, run);
    }
    if opts.dry_run {
        return record_replacement(path, count, run);
    }
    let replaced = build_replaced(content, first, projected, opts.find, opts.replace_with);
    match fsops::write_replaced(path, replaced) {
        Ok(()) => record_replacement(path, count, run),
        Err(e) => record_error(path, &e.to_string(), run),
    }
}

fn build_replaced(
    content: &str,
    first: usize,
    projected: usize,
    find: &str,
    replace_with: &str,
) -> String {
    let mut out = String::with_capacity(projected);
    out.push_str(content.get(..first).unwrap_or_default());
    out.push_str(replace_with);
    let mut tail = content.get(first + find.len()..).unwrap_or_default();
    while let Some(pos) = tail.find(find) {
        out.push_str(tail.get(..pos).unwrap_or_default());
        out.push_str(replace_with);
        tail = tail.get(pos + find.len()..).unwrap_or_default();
    }
    out.push_str(tail);
    out
}

fn record_error(path: &Path, message: &str, run: &mut ReplaceRun) {
    run.errors
        .push(json!({ "path": path.display().to_string(), "error": message }));
}

fn record_replacement(path: &Path, count: usize, run: &mut ReplaceRun) {
    run.total += count;
    run.changed.push(json!({
        "path": path.display().to_string(),
        "replacements": count,
    }));
}

fn replacement_report(root: &Path, params: &ReplaceParams, run: &ReplaceRun) -> CallToolResult {
    json_response(&json!({
        "root": root.display().to_string(),
        "find": params.find,
        "replace": params.replace,
        "dry_run": params.dry_run,
        "files_affected": run.changed.len(),
        "total_replacements": run.total,
        "files": run.changed,
        "errors": run.errors.len(),
        "error_details": run.errors,
    }))
}

fn matches_file(name: &str, opts: &ReplaceOpts<'_>) -> bool {
    let ext_ok = opts.ext_filter.is_none_or(|ext| {
        Path::new(name)
            .extension()
            .and_then(|e| e.to_str())
            .is_some_and(|e| e.eq_ignore_ascii_case(ext))
    });
    ext_ok && opts.name_pattern.is_none_or(|p| p.matches(name))
}
