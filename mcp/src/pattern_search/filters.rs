//! Purpose: `find`: a walk that reports entries passing its name, type, size,
//! depth and age filters, never descending through a link.

use crate::conversions::format_size;
use crate::globmatch::CompiledGlob;
use crate::tools::{json_response, patterns::FindParams};
use crate::FfmServer;
use rmcp::{model::CallToolResult, ErrorData as McpError};
use serde_json::json;
use std::{fs, path::Path, time::SystemTime};

#[cfg(test)]
mod tests;

pub fn find(server: &FfmServer, params: &FindParams) -> Result<CallToolResult, McpError> {
    let root = server.safe_path(&params.path)?;
    if !root.is_dir() {
        return Err(McpError::invalid_params(
            format!("not a directory: {}", root.display()),
            None,
        ));
    }
    let compiled = params
        .name
        .as_deref()
        .map(CompiledGlob::compile)
        .transpose()
        .map_err(|e| McpError::invalid_params(e, None))?;

    let mut run = FindRun {
        params,
        name_glob: compiled.as_ref(),
        now: SystemTime::now(),
        results: Vec::new(),
        visited: 0,
    };
    find_recursive(&root, 0, &mut run);

    let result = json!({
        "root": root.display().to_string(),
        "count": run.results.len(),
        "results": run.results,
    });
    Ok(json_response(&result))
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum FindEntryKind {
    File,
    Directory,
    Symlink,
}

impl FindEntryKind {
    fn of(entry: &fs::DirEntry) -> Self {
        entry.file_type().map_or(Self::File, |ft| {
            if ft.is_symlink() {
                Self::Symlink
            } else if ft.is_dir() {
                Self::Directory
            } else {
                Self::File
            }
        })
    }

    const fn label(self) -> &'static str {
        match self {
            Self::File => "file",
            Self::Directory => "dir",
            Self::Symlink => "symlink",
        }
    }
}

struct FindRun<'a> {
    params: &'a FindParams,
    name_glob: Option<&'a CompiledGlob<'a>>,
    now: SystemTime,
    results: Vec<serde_json::Value>,
    visited: usize,
}

impl FindRun<'_> {
    fn budget_reached(&self) -> bool {
        self.results.len() >= self.params.max_results || self.visited >= crate::walk::WALK_BUDGET
    }

    fn matching_size(&self, entry: &fs::DirEntry, name: &str, kind: FindEntryKind) -> Option<u64> {
        let name_ok = self.name_glob.is_none_or(|glob| glob.matches(name));
        let kind_ok = self
            .params
            .file_type
            .as_deref()
            .is_none_or(|wanted| wanted == kind.label());
        if !name_ok || !kind_ok {
            return None;
        }
        let meta = entry.metadata().ok()?;
        matches_metadata(&meta, self.params, self.now).then_some(meta.len())
    }
}

fn find_recursive(dir: &Path, depth: usize, run: &mut FindRun<'_>) {
    if run.budget_reached() {
        return;
    }
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        if run.budget_reached() {
            return;
        }
        run.visited += 1;
        visit_find_entry(&entry, depth, run);
    }
}

fn matches_metadata(meta: &fs::Metadata, params: &FindParams, now: SystemTime) -> bool {
    if params.min_size.is_some_and(|min| meta.len() < min)
        || params.max_size.is_some_and(|max| meta.len() > max)
    {
        return false;
    }
    let Some(secs) = params.modified_within_secs else {
        return true;
    };
    modified_within(&meta.modified(), now, secs)
}

/// Missing or future timestamps cannot prove that an entry is too old.
fn modified_within(modified: &std::io::Result<SystemTime>, now: SystemTime, secs: u64) -> bool {
    let Ok(modified) = modified else {
        return true;
    };
    let Ok(elapsed) = now.duration_since(*modified) else {
        return true;
    };
    elapsed.as_secs() <= secs
}

fn visit_find_entry(entry: &fs::DirEntry, depth: usize, run: &mut FindRun<'_>) {
    let kind = FindEntryKind::of(entry);
    let raw_name = entry.file_name();
    let name = raw_name.to_string_lossy();
    let matched_size = run.matching_size(entry, &name, kind);
    let descend =
        kind == FindEntryKind::Directory && run.params.max_depth.is_none_or(|max| depth < max);
    if matched_size.is_some() || descend {
        let path = entry.path();
        if let Some(size) = matched_size {
            push_find_match(&path, &name, kind.label(), size, &mut run.results);
        }
        if descend {
            find_recursive(&path, depth + 1, run);
        }
    }
}

fn push_find_match(
    path: &Path,
    name: &str,
    kind: &str,
    size: u64,
    results: &mut Vec<serde_json::Value>,
) {
    results.push(json!({
        "path": path.display().to_string(),
        "name": name,
        "type": kind,
        "size": size,
        "size_human": format_size(size),
    }));
}
