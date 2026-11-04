//! Root-confined content search and its bounded result scan.

use super::{json_response, walkable_type, FfmServer, GrepParams};
use rmcp::{model::CallToolResult, ErrorData as McpError};
use serde_json::json;
use std::io;
use std::{fs, path::Path};

#[cfg(test)]
mod tests;

pub fn grep(server: &FfmServer, params: &GrepParams) -> Result<CallToolResult, McpError> {
    let path = server.safe_path(&params.path)?;
    if !path.is_dir() {
        return Err(McpError::invalid_params(
            format!("not a directory: {}", path.display()),
            None,
        ));
    }
    let pattern_lower = params.pattern.to_lowercase();
    let mut results = Vec::new();
    let mut scratch = String::new();
    let mut visited = 0usize;

    grep_recursive(
        &path,
        &pattern_lower,
        params.extension.as_deref(),
        &mut scratch,
        &mut results,
        params.max_results,
        &mut visited,
    );

    let result = json!({
        "pattern": params.pattern,
        "root": path.display().to_string(),
        "count": results.len(),
        "matches": results,
    });
    Ok(json_response(&result))
}

fn grep_recursive(
    dir: &Path,
    pattern: &str,
    ext_filter: Option<&str>,
    scratch: &mut String,
    results: &mut Vec<serde_json::Value>,
    max: usize,
    visited: &mut usize,
) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        if results.len() >= max || *visited >= crate::walk::WALK_BUDGET {
            return;
        }
        *visited += 1;
        let Some(file_type) = walkable_type(&entry) else {
            continue;
        };
        if file_type.is_dir() {
            grep_recursive(
                &entry.path(),
                pattern,
                ext_filter,
                scratch,
                results,
                max,
                visited,
            );
            continue;
        }
        if searchable_entry(&entry, file_type, ext_filter) {
            grep_file(&entry.path(), pattern, scratch, results, max);
        }
    }
}

/// Append matches from one file, stopping on a decoding error or result limit.
fn grep_file(
    path: &Path,
    pattern: &str,
    scratch: &mut String,
    results: &mut Vec<serde_json::Value>,
    max: usize,
) {
    use std::io::BufRead;

    let Ok(file) = fs::File::open(path) else {
        return;
    };
    let mut reader = io::BufReader::new(file);
    let mut line_num = 0usize;
    let mut display_path: Option<String> = None;
    while results.len() < max {
        scratch.clear();
        let Ok(bytes) = reader.read_line(scratch) else {
            break;
        };
        if bytes == 0 {
            break;
        }
        line_num += 1;
        if contains_lowercased(scratch, pattern) {
            let file_label = display_path.get_or_insert_with(|| path.display().to_string());
            results.push(json!({
                "file": file_label.as_str(),
                "line": line_num,
                "content": scratch.trim(),
            }));
        }
    }
}

pub fn contains_lowercased(haystack: &str, needle_lower: &str) -> bool {
    if needle_lower.is_empty() {
        return true;
    }
    if haystack.is_ascii() && needle_lower.is_ascii() {
        return contains_ascii_lowercased(haystack.as_bytes(), needle_lower.as_bytes());
    }
    if let Some(anchor) = safe_ascii_anchor(needle_lower.as_bytes()) {
        let up = anchor.to_ascii_uppercase();
        if !haystack.as_bytes().iter().any(|&b| b == anchor || b == up) {
            return false;
        }
    }
    haystack.to_lowercase().contains(needle_lower)
}

fn contains_ascii_lowercased(haystack: &[u8], needle_lower: &[u8]) -> bool {
    let Some(max_start) = haystack.len().checked_sub(needle_lower.len()) else {
        return false;
    };
    let first = needle_lower.first().copied().unwrap_or_default();
    let upper = first.to_ascii_uppercase();
    let search = haystack.get(..=max_start).unwrap_or_default();
    for (idx, &byte) in search.iter().enumerate() {
        if (byte == first || byte == upper)
            && haystack
                .get(idx..idx + needle_lower.len())
                .is_some_and(|w| w.eq_ignore_ascii_case(needle_lower))
        {
            return true;
        }
    }
    false
}

fn safe_ascii_anchor(needle_lower: &[u8]) -> Option<u8> {
    needle_lower
        .iter()
        .copied()
        .find(|&b| b.is_ascii() && !matches!(b, b'i' | b'k'))
}

#[expect(
    clippy::filetype_is_file,
    reason = "skip FIFOs, sockets and device nodes"
)]
fn searchable_entry(
    entry: &fs::DirEntry,
    file_type: fs::FileType,
    extension: Option<&str>,
) -> bool {
    if let Some(ext) = extension {
        let name = entry.file_name();
        let matches_ext = Path::new(&name)
            .extension()
            .and_then(|value| value.to_str())
            .is_some_and(|value| value.eq_ignore_ascii_case(ext));
        if !matches_ext {
            return false;
        }
    }
    file_type.is_file() && entry.metadata().map_or(0, |meta| meta.len()) <= 10 * 1024 * 1024
}
