//! Root-confined content search and its bounded result scan.

use super::{json_response, walkable_type, FfmServer, GrepParams};
use rmcp::{model::CallToolResult, ErrorData as McpError};
use serde_json::json;
use std::io;
use std::{fs, path::Path};

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
    let mut visited = 0usize;

    grep_recursive(
        &path,
        &pattern_lower,
        params.extension.as_deref(),
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
        // A link is never opened: `File::open` below would follow it out
        // of the server root (see `walkable_type`).
        let Some(file_type) = walkable_type(&entry) else {
            continue;
        };
        let path = entry.path();
        if file_type.is_dir() {
            grep_recursive(&path, pattern, ext_filter, results, max, visited);
            continue;
        }
        #[expect(
            clippy::filetype_is_file,
            reason = "skip FIFOs, sockets and device nodes"
        )]
        if file_type.is_file() && searchable_file(&path, ext_filter) {
            grep_file(&path, pattern, results, max);
        }
    }
}

/// Append matches from one file, stopping on a decoding error or result limit.
fn grep_file(path: &Path, pattern: &str, results: &mut Vec<serde_json::Value>, max: usize) {
    use std::io::BufRead;

    let Ok(file) = fs::File::open(path) else {
        return;
    };
    let mut reader = io::BufReader::new(file);
    let mut line = String::new();
    let mut line_num = 0usize;
    while results.len() < max {
        line.clear();
        let Ok(bytes) = reader.read_line(&mut line) else {
            break;
        };
        if bytes == 0 {
            break;
        }
        line_num += 1;
        if contains_lowercased(&line, pattern) {
            results.push(json!({
                "file": path.display().to_string(),
                "line": line_num,
                "content": line.trim(),
            }));
        }
    }
}

fn contains_lowercased(haystack: &str, needle_lower: &str) -> bool {
    if needle_lower.is_empty() {
        return true;
    }
    if haystack.is_ascii() && needle_lower.is_ascii() {
        let (haystack, needle) = (haystack.as_bytes(), needle_lower.as_bytes());
        return haystack.len() >= needle.len()
            && haystack
                .windows(needle.len())
                .any(|window| window.eq_ignore_ascii_case(needle));
    }
    haystack.to_lowercase().contains(needle_lower)
}

fn searchable_file(path: &Path, extension: Option<&str>) -> bool {
    if extension.is_some_and(|ext| {
        !path
            .extension()
            .and_then(|value| value.to_str())
            .unwrap_or("")
            .eq_ignore_ascii_case(ext)
    }) {
        return false;
    }
    fs::metadata(path).map_or(0, |meta| meta.len()) <= 10 * 1024 * 1024
}
