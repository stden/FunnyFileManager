//! Purpose: Glob patterns and filters: glob, find, replace in files, and
//! delete by pattern.

use super::{default_max_results, default_true};
use crate::{cleanup, pattern_search, replace, FfmServer};
use rmcp::{
    handler::server::wrapper::Parameters, model::CallToolResult, tool, tool_router,
    ErrorData as McpError,
};
use serde::Deserialize;

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct GlobParams {
    /// Root directory to search in
    pub path: String,
    /// Glob pattern (e.g. "*.rs", "**/*.txt", "src/**/*.json")
    pub pattern: String,
    /// Maximum results (default: 100)
    #[serde(default = "default_max_results")]
    pub max_results: usize,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct FindParams {
    /// Root directory to search in
    pub path: String,
    /// Name glob matched against the entry name (e.g. "*.rs", "mod?.rs").
    /// Without a wildcard it must equal the name — optional
    #[serde(default)]
    pub name: Option<String>,
    /// Filter by type: "file", "dir", or "symlink" — optional
    #[serde(default)]
    pub file_type: Option<String>,
    /// Minimum file size in bytes — optional
    #[serde(default)]
    pub min_size: Option<u64>,
    /// Maximum file size in bytes — optional
    #[serde(default)]
    pub max_size: Option<u64>,
    /// Maximum directory depth to search (default: unlimited)
    #[serde(default)]
    pub max_depth: Option<usize>,
    /// Only files modified within the last N seconds — optional
    #[serde(default)]
    pub modified_within_secs: Option<u64>,
    /// Maximum results (default: 100)
    #[serde(default = "default_max_results")]
    pub max_results: usize,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct ReplaceParams {
    /// Root directory to search in
    pub path: String,
    /// Text to find (case-sensitive)
    pub find: String,
    /// Text to replace with
    pub replace: String,
    /// File extension filter (e.g. "rs", "txt") — optional
    #[serde(default)]
    pub extension: Option<String>,
    /// Glob pattern for filenames (e.g. "*.rs") — optional
    #[serde(default)]
    pub name_pattern: Option<String>,
    /// Perform a dry run without modifying files (default: false)
    #[serde(default)]
    pub dry_run: bool,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct DeleteByPatternParams {
    /// Root directory to search in
    pub path: String,
    /// Glob pattern for files to delete (e.g. "*.tmp", "*.bak")
    pub pattern: String,
    /// Also delete matching directories (default: false)
    #[serde(default)]
    pub include_dirs: bool,
    /// Recursive — search subdirectories (default: true)
    #[serde(default = "default_true")]
    pub recursive: bool,
    /// Perform a dry run without deleting (default: false)
    #[serde(default)]
    pub dry_run: bool,
}

#[tool_router(router = pattern_tools, vis = "pub(super)")]
impl FfmServer {
    /// Find files matching a glob pattern (e.g. "*.rs", "**/*.txt").
    #[tool(
        description = "Find files matching a glob pattern, relative to the search root (e.g. '*.rs', '**/*.txt', 'src/**/*.json'). '**' spans directories, '*' and '?' stay within one path component."
    )]
    pub(crate) fn glob(
        &self,
        Parameters(params): Parameters<GlobParams>,
    ) -> Result<CallToolResult, McpError> {
        pattern_search::glob(self, &params)
    }

    /// Advanced find with filters: name pattern, file type, size range, depth, modification time.
    #[tool(
        description = "Advanced file finder with filters: name glob, type (file/dir/symlink), size range, max depth, modification time."
    )]
    pub(crate) fn find(
        &self,
        Parameters(params): Parameters<FindParams>,
    ) -> Result<CallToolResult, McpError> {
        pattern_search::find(self, &params)
    }

    /// Find and replace text across files in a directory tree.
    #[tool(
        description = "Find and replace text in files across a directory tree. Supports extension and name filters. Use dry_run=true to preview."
    )]
    pub(crate) fn replace(
        &self,
        Parameters(params): Parameters<ReplaceParams>,
    ) -> Result<CallToolResult, McpError> {
        replace::replace(self, &params)
    }

    /// Delete files matching a glob pattern (e.g. "*.tmp", "*.bak").
    #[tool(
        description = "Delete files matching a glob pattern (e.g. '*.tmp', '*.bak'). Use dry_run=true to preview. Searches recursively by default."
    )]
    pub(crate) fn delete_by_pattern(
        &self,
        Parameters(params): Parameters<DeleteByPatternParams>,
    ) -> Result<CallToolResult, McpError> {
        cleanup::delete_by_pattern(self, &params)
    }
}
