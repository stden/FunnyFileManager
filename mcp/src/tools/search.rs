//! Purpose: Finding entries by name, file contents by text, and drawing a
//! directory tree.

use super::{default_max_results, json_response};
use crate::{content_search, FfmServer};
use ffm_mcp::tree_walk::render_directory_tree;
use rmcp::{
    handler::server::wrapper::Parameters, model::CallToolResult, tool, tool_router,
    ErrorData as McpError,
};
use serde::Deserialize;
use serde_json::json;
use std::fs;
use std::path::Path;

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct SearchParams {
    /// Root directory to search in
    pub path: String,
    /// Name pattern to match (case-insensitive substring)
    pub pattern: String,
    /// Maximum results to return (default: 100)
    #[serde(default = "default_max_results")]
    pub max_results: usize,
    /// Only return files
    #[serde(default)]
    pub files_only: bool,
    /// Only return directories
    #[serde(default)]
    pub dirs_only: bool,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct TreeParams {
    /// Root directory
    pub path: String,
    /// Max depth (default: 3)
    #[serde(default = "default_depth")]
    pub depth: usize,
}

fn default_depth() -> usize {
    3
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct GrepParams {
    /// Directory to search in
    pub path: String,
    /// Text pattern to search for (case-insensitive)
    pub pattern: String,
    /// File extension filter (e.g. "rs", "txt")
    #[serde(default)]
    pub extension: Option<String>,
    /// Maximum results (default: 50)
    #[serde(default = "default_grep_max")]
    pub max_results: usize,
}

fn default_grep_max() -> usize {
    50
}

#[tool_router(router = search_tools, vis = "pub(super)")]
impl FfmServer {
    /// Search for files/directories matching a pattern within a directory tree.
    #[tool(
        description = "Search for files/directories matching a name pattern within a directory tree."
    )]
    pub(crate) fn search(
        &self,
        Parameters(params): Parameters<SearchParams>,
    ) -> Result<CallToolResult, McpError> {
        let path = self.safe_path(&params.path)?;
        let pattern_lower = params.pattern.to_lowercase();
        let mut results = Vec::new();

        search_recursive(
            &path,
            &pattern_lower,
            params.files_only,
            params.dirs_only,
            &mut results,
            params.max_results,
        );

        let result = json!({
            "pattern": params.pattern,
            "root": path.display().to_string(),
            "count": results.len(),
            "results": results,
        });
        Ok(json_response(&result))
    }

    /// Search file contents for a text pattern (like grep).
    #[tool(
        description = "Search file contents for a text pattern (case-insensitive grep). Returns matching lines with file paths and line numbers."
    )]
    pub(crate) fn grep(
        &self,
        Parameters(params): Parameters<GrepParams>,
    ) -> Result<CallToolResult, McpError> {
        content_search::grep(self, &params)
    }

    /// Show directory tree structure up to a given depth.
    #[tool(description = "Show directory tree structure up to a given depth.")]
    pub(crate) fn tree(
        &self,
        Parameters(params): Parameters<TreeParams>,
    ) -> Result<CallToolResult, McpError> {
        let path = self.safe_dir(&params.path)?;

        let tree = render_directory_tree(&path, params.depth);

        let result = json!({
            "path": path.display().to_string(),
            "depth": params.depth,
            "tree": tree,
        });
        Ok(json_response(&result))
    }
}

fn search_recursive(
    dir: &Path,
    pattern: &str,
    files_only: bool,
    dirs_only: bool,
    results: &mut Vec<serde_json::Value>,
    max: usize,
) {
    if results.len() >= max {
        return;
    }
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        if results.len() >= max {
            return;
        }
        let name = entry.file_name().to_string_lossy().to_string();
        let is_dir = entry.file_type().is_ok_and(|t| t.is_dir());

        if name.to_lowercase().contains(pattern) {
            let include = (!files_only || !is_dir) && (!dirs_only || is_dir);
            if include {
                results.push(json!({
                    "path": entry.path().display().to_string(),
                    "name": name,
                    "type": if is_dir { "dir" } else { "file" },
                }));
            }
        }
        if is_dir {
            search_recursive(&entry.path(), pattern, files_only, dirs_only, results, max);
        }
    }
}
