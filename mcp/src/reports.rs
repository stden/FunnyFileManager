//! Directory composition and duplicate-content reports.

use super::{
    checksum_of, collect_files, content_kind, format_size, json_response, walkable_type,
    AnalyseParams, CollectedFile, DuplicatesParams, FfmServer, WALK_BUDGET,
};
use rmcp::{model::CallToolResult, ErrorData as McpError};
use serde_json::json;
use std::collections::BTreeMap;
use std::fs;

pub(super) fn analyse_directory(
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

    let (by_kind, total) = classify_files(&files);
    let children = child_directories(&root, &files);

    let truncated = files.len() >= WALK_BUDGET;
    let mut largest = files;
    largest.sort_by_key(|file| std::cmp::Reverse(file.size));
    largest.truncate(params.largest.unwrap_or(10).min(1_000));

    let result = json!({
        "path": root.display().to_string(),
        "total_size": total,
        "total_size_human": format_size(total),
        "file_count": by_kind.values().map(|(_, count)| count).sum::<usize>(),
        "truncated": truncated,
        "subdirectories": children,
        "by_kind": kind_report(&by_kind),
        "largest_files": file_report(&largest),
    });
    Ok(json_response(&result))
}

pub(super) fn find_duplicates(
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

type KindTotals = BTreeMap<&'static str, (u64, usize)>;

fn classify_files(files: &[CollectedFile]) -> (KindTotals, u64) {
    let mut kinds = KindTotals::new();
    let mut total = 0;
    for file in files {
        total += file.size;
        let entry = kinds.entry(content_kind(&file.name)).or_default();
        entry.0 += file.size;
        entry.1 += 1;
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
    let mut children: Vec<_> = sizes
        .into_iter()
        .map(|(name, size)| json!({"name": name, "size": size, "size_human": format_size(size)}))
        .collect();
    children.sort_by_key(|child| std::cmp::Reverse(child["size"].as_u64().unwrap_or(0)));
    children
}

fn duplicate_groups(files: &[CollectedFile], min_size: u64) -> (Vec<serde_json::Value>, u64) {
    let mut by_size: BTreeMap<u64, Vec<&CollectedFile>> = BTreeMap::new();
    for file in files.iter().filter(|file| file.size >= min_size) {
        by_size.entry(file.size).or_default().push(file);
    }

    let mut groups = Vec::new();
    let mut wasted = 0u64;
    for (size, candidates) in by_size.iter().filter(|(_, group)| group.len() > 1) {
        let mut by_hash: BTreeMap<String, Vec<&str>> = BTreeMap::new();
        for file in candidates {
            if let Some(hash) = checksum_of(&file.path) {
                by_hash.entry(hash).or_default().push(&file.relative);
            }
        }
        for (hash, paths) in by_hash.into_iter().filter(|(_, paths)| paths.len() > 1) {
            wasted += size * (paths.len() as u64 - 1);
            groups.push(json!({
                "checksum": hash,
                "size": size,
                "size_human": format_size(*size),
                "copies": paths.len(),
                "paths": paths,
            }));
        }
    }
    groups.sort_by_key(|group| std::cmp::Reverse(group["size"].as_u64().unwrap_or(0)));

    (groups, wasted)
}

fn kind_report(kinds: &KindTotals) -> Vec<serde_json::Value> {
    kinds
        .iter()
        .map(|(kind, (size, count))| {
            json!({
                "kind": kind, "size": size, "size_human": format_size(*size), "files": count,
            })
        })
        .collect()
}

fn file_report(files: &[CollectedFile]) -> Vec<serde_json::Value> {
    files
        .iter()
        .map(|file| {
            json!({
                "path": file.relative, "size": file.size, "size_human": format_size(file.size),
            })
        })
        .collect()
}
