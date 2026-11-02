//! Purpose: Where the space went and what changed: directory composition,
//! duplicates, the largest files and recently modified ones.

use super::json_response;
use crate::walk::{collect_files, CollectedFile, WALK_BUDGET};
use crate::{conversions::format_size, reports, FfmServer};
use rmcp::{
    handler::server::wrapper::Parameters, model::CallToolResult, tool, tool_router,
    ErrorData as McpError,
};
use serde::Deserialize;
use serde_json::json;
use std::fs;
use std::time::{Duration, SystemTime};

/// A JSON `days` value is unbounded; the window is capped at a century.
const MAX_RECENT_DAYS: u64 = 36_500;

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct AnalyseParams {
    /// Absolute path to the directory to measure
    pub path: String,
    /// How many of the largest files to name (default 10)
    #[serde(default)]
    pub largest: Option<usize>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct DuplicatesParams {
    /// Absolute path to the directory to search
    pub path: String,
    /// Ignore files smaller than this many bytes (default 1)
    #[serde(default)]
    pub min_size: Option<u64>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct LargestParams {
    /// Absolute path to the directory to search
    pub path: String,
    /// How many files to report (default 20)
    #[serde(default)]
    pub limit: Option<usize>,
    /// Ignore files smaller than this many bytes (default 1)
    #[serde(default)]
    pub min_size: Option<u64>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct RecentParams {
    /// Absolute path to the directory to search
    pub path: String,
    /// Only files modified within this many days (default 7)
    #[serde(default)]
    pub days: Option<u64>,
    /// How many to return, newest first (default 50)
    #[serde(default)]
    pub limit: Option<usize>,
}

#[tool_router(router = analysis_tools, vis = "pub(super)")]
impl FfmServer {
    /// Where a directory's space has gone, in one call.
    #[tool(
        description = "Analyse a directory: total size, size of each immediate subdirectory, totals per file kind (source, document, image, audio, video, archive, program, data), and the largest files. Answers 'what is taking up the space' without walking the tree by hand."
    )]
    pub(crate) fn analyse_directory(
        &self,
        Parameters(params): Parameters<AnalyseParams>,
    ) -> Result<CallToolResult, McpError> {
        reports::analyse_directory(self, &params)
    }

    /// Files with identical content, grouped.
    #[tool(
        description = "Find files with identical content under a directory, grouped by checksum. Only files of the same size are hashed, so a tree of unique files costs almost nothing."
    )]
    pub(crate) fn find_duplicates(
        &self,
        Parameters(params): Parameters<DuplicatesParams>,
    ) -> Result<CallToolResult, McpError> {
        reports::find_duplicates(self, &params)
    }

    /// What changed lately.
    #[tool(
        description = "List files under a directory modified within the last N days, newest first. Answers 'what have I been working on' and 'what did that build touch'."
    )]
    pub(crate) fn recent_files(
        &self,
        Parameters(params): Parameters<RecentParams>,
    ) -> Result<CallToolResult, McpError> {
        let root = self.safe_dir(&params.path)?;
        let days = params.days.unwrap_or(7).min(MAX_RECENT_DAYS);
        let cutoff = SystemTime::now()
            .checked_sub(Duration::from_secs(days.saturating_mul(86_400)))
            .unwrap_or(SystemTime::UNIX_EPOCH);

        let mut files = Vec::new();
        collect_files(&root, &root, &mut files, WALK_BUDGET);

        let recent = recent_entries(&files, cutoff, params.limit.unwrap_or(50).min(10_000));

        let result = json!({
            "path": root.display().to_string(),
            "days": days,
            "count": recent.len(),
            "files": recent
                .iter()
                .map(|(modified, file)| json!({
                    "path": file.relative,
                    "size": file.size,
                    "size_human": format_size(file.size),
                    "modified_unix": modified
                        .duration_since(SystemTime::UNIX_EPOCH)
                        .map_or(0, |since| since.as_secs()),
                }))
                .collect::<Vec<_>>(),
        });
        Ok(json_response(&result))
    }

    /// What is taking up the space, which `disk_usage` totals but does not name.
    #[tool(
        description = "List the largest files under a directory, biggest first. Answers 'what is taking up the space', which disk_usage only totals."
    )]
    pub(crate) fn largest_files(
        &self,
        Parameters(params): Parameters<LargestParams>,
    ) -> Result<CallToolResult, McpError> {
        let root = self.safe_dir(&params.path)?;
        let limit = params.limit.unwrap_or(20);
        let min_size = params.min_size.unwrap_or(1);

        let mut files = Vec::new();
        collect_files(&root, &root, &mut files, WALK_BUDGET);
        // The same signal analyse_directory gives: a walk the budget stopped
        // early is a partial ranking, and a silent one misleads the caller.
        // Judged before the size filter, which hides how far the walk got.
        let truncated = files.len() >= WALK_BUDGET;
        files.retain(|file| file.size >= min_size);
        // Biggest first, and by path within a size so the answer is stable
        // between runs over the same tree.
        files.sort_by(|a, b| {
            b.size
                .cmp(&a.size)
                .then_with(|| a.relative.cmp(&b.relative))
        });
        let total: u64 = files.iter().map(|file| file.size).sum();
        // Both counted before the truncation, so the caller can tell a top-20
        // of twenty files from a top-20 of twenty thousand.
        let considered = files.len();
        files.truncate(limit);

        let result = json!({
            "path": root.display().to_string(),
            "files_considered": considered,
            "total_bytes": total,
            "truncated": truncated,
            "files": files
                .iter()
                .map(|file| json!({
                    "path": file.relative,
                    "size": file.size,
                    "size_human": format_size(file.size),
                }))
                .collect::<Vec<_>>(),
        });
        Ok(json_response(&result))
    }
}

fn recent_entries(
    files: &[CollectedFile],
    cutoff: SystemTime,
    limit: usize,
) -> Vec<(SystemTime, &CollectedFile)> {
    let mut recent: Vec<(SystemTime, &CollectedFile)> = files
        .iter()
        .filter_map(|file| {
            let modified = fs::metadata(&file.path).ok()?.modified().ok()?;
            (modified >= cutoff).then_some((modified, file))
        })
        .collect();
    recent.sort_by_key(|(modified, _)| std::cmp::Reverse(*modified));
    recent.truncate(limit);

    recent
}
