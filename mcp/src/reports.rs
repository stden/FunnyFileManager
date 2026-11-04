//! Purpose: Directory composition and duplicate-content reports.

use super::{
    collect_files, content_kind, format_size, json_response, walkable_type, AnalyseParams,
    CollectedFile, DuplicatesParams, FfmServer, WALK_BUDGET,
};
use duplicates::duplicate_groups;
use rmcp::{model::CallToolResult, ErrorData as McpError};
use serde_json::json;
use std::collections::BTreeMap;
use std::fs;

mod duplicates;
#[cfg(test)]
mod tests;

pub fn analyse_directory(
    server: &FfmServer,
    params: &AnalyseParams,
) -> Result<CallToolResult, McpError> {
    let root = server.safe_path(&params.path)?;
    if !root.is_dir() {
        return Err(McpError::invalid_params(
            format!("not a directory: {}", root.display()),
            None,
        ));
    }

    let mut files = Vec::new();
    collect_files(&root, &root, &mut files, WALK_BUDGET);

    let file_count = files.len();
    let truncated = file_count >= WALK_BUDGET;
    let (by_kind, total) = classify_files(&files);
    let children = child_directories(&root, &files);
    let largest = select_largest_files(files, params.largest.unwrap_or(10).min(1_000));

    let result = json!({
        "path": root.display().to_string(),
        "total_size": total,
        "total_size_human": format_size(total),
        "file_count": file_count,
        "truncated": truncated,
        "subdirectories": children,
        "by_kind": kind_report(&by_kind),
        "largest_files": file_report(&largest),
    });
    Ok(json_response(&result))
}

fn select_largest_files(mut files: Vec<CollectedFile>, cap: usize) -> Vec<CollectedFile> {
    if files.len() > cap && cap > 0 {
        files.select_nth_unstable_by_key(cap - 1, |file| std::cmp::Reverse(file.size));
    }
    files.truncate(cap);
    files.sort_by_key(|file| std::cmp::Reverse(file.size));
    files
}

pub fn find_duplicates(
    server: &FfmServer,
    params: &DuplicatesParams,
) -> Result<CallToolResult, McpError> {
    let root = server.safe_path(&params.path)?;
    if !root.is_dir() {
        return Err(McpError::invalid_params(
            format!("not a directory: {}", root.display()),
            None,
        ));
    }
    let min_size = params.min_size.unwrap_or(1);

    let mut files = Vec::new();
    collect_files(&root, &root, &mut files, WALK_BUDGET);

    // Same size first: hashing every file to find that none match would
    // read the whole tree for nothing, and size is free.
    let (groups, wasted) = duplicate_groups(&files, min_size);

    let result = json!({
        "path": root.display().to_string(),
        "duplicate_groups": groups.len(),
        "reclaimable": wasted,
        "reclaimable_human": format_size(wasted),
        "groups": groups,
    });
    Ok(json_response(&result))
}

#[derive(Default)]
struct KindStats {
    bytes: u64,
    files: usize,
}

type KindTotals = BTreeMap<&'static str, KindStats>;

fn classify_files(files: &[CollectedFile]) -> (KindTotals, u64) {
    let mut kinds = KindTotals::new();
    let mut total = 0;
    for file in files {
        total += file.size;
        let stats = kinds.entry(content_kind(&file.name)).or_default();
        stats.bytes += file.size;
        stats.files += 1;
    }
    (kinds, total)
}

fn child_directories(root: &std::path::Path, files: &[CollectedFile]) -> Vec<serde_json::Value> {
    let mut sizes = BTreeMap::new();
    for file in files {
        if let Some((head, _)) = file.relative.split_once('/') {
            *sizes.entry(head.to_owned()).or_insert(0) += file.size;
        }
    }
    if let Ok(entries) = fs::read_dir(root) {
        for entry in entries.flatten() {
            if walkable_type(&entry).is_some_and(|kind| kind.is_dir()) {
                sizes
                    .entry(entry.file_name().to_string_lossy().into_owned())
                    .or_insert(0);
            }
        }
    }
    let mut ranked: Vec<(String, u64)> = sizes.into_iter().collect();
    ranked.sort_by_key(|&(_, size)| std::cmp::Reverse(size));
    ranked
        .into_iter()
        .map(|(name, size)| json!({"name": name, "size": size, "size_human": format_size(size)}))
        .collect()
}

fn kind_report(kinds: &KindTotals) -> Vec<serde_json::Value> {
    kinds
        .iter()
        .map(|(kind, stats)| {
            json!({
                "kind": kind,
                "size": stats.bytes,
                "size_human": format_size(stats.bytes),
                "files": stats.files,
            })
        })
        .collect()
}

fn file_report(files: &[CollectedFile]) -> Vec<serde_json::Value> {
    files
        .iter()
        .map(|f| json!({"path": f.relative, "size": f.size, "size_human": format_size(f.size)}))
        .collect()
}
