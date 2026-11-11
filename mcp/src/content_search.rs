//! Root-confined content search and its bounded result scan.

mod pattern;

use super::{json_response, walkable_type, FfmServer, GrepParams};
use pattern::PreparedPattern;
use rmcp::{model::CallToolResult, ErrorData as McpError};
use serde_json::json;
use std::io;
use std::{fs, path::Path};

#[cfg(test)]
mod tests;

#[derive(Default)]
struct GrepScratch {
    line: String,
    lower: String,
}

pub fn grep(server: &FfmServer, params: &GrepParams) -> Result<CallToolResult, McpError> {
    let path = server.safe_path(&params.path)?;
    if !path.is_dir() {
        return Err(McpError::invalid_params(
            format!("not a directory: {}", path.display()),
            None,
        ));
    }
    let prepared = PreparedPattern::new(&params.pattern.to_lowercase());
    let mut results = Vec::new();
    let mut scratch = GrepScratch::default();
    let mut visited = 0usize;

    grep_recursive(
        &path,
        &prepared,
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
    pattern: &PreparedPattern,
    ext_filter: Option<&str>,
    scratch: &mut GrepScratch,
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
    pattern: &PreparedPattern,
    scratch: &mut GrepScratch,
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
        scratch.line.clear();
        let Ok(bytes) = reader.read_line(&mut scratch.line) else {
            break;
        };
        if bytes == 0 {
            break;
        }
        line_num += 1;
        if pattern.matches(&scratch.line, &mut scratch.lower) {
            let file_label = display_path.get_or_insert_with(|| path.display().to_string());
            results.push(json!({
                "file": file_label.as_str(),
                "line": line_num,
                "content": scratch.line.trim(),
            }));
        }
    }
}

pub fn contains_lowercased(haystack: &str, needle_lower: &str) -> bool {
    PreparedPattern::new(needle_lower).matches(haystack, &mut String::new())
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
