//! Purpose: Comparing two directories by name and size, and two files byte
//! for byte.

use super::json_response;
use crate::walk::{collect_files, WALK_BUDGET};
use crate::FfmServer;
use rmcp::{
    handler::server::wrapper::Parameters, model::CallToolResult, tool, tool_router,
    ErrorData as McpError,
};
use serde::Deserialize;
use serde_json::json;
use std::collections::BTreeMap;
use std::fs;
use std::io;
use std::path::Path;
#[cfg(test)]
use stream::PairDigests;
use stream::{compare_streams, ComparisonOutcome};

mod stream;
#[cfg(test)]
mod tests;

pub(super) struct FilePairComparison {
    left_size: u64,
    right_size: u64,
    outcome: ComparisonOutcome,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct CompareFilesParams {
    /// Absolute path to the first file
    pub left: String,
    /// Absolute path to the second file
    pub right: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct CompareParams {
    /// Absolute path to the first directory
    pub left: String,
    /// Absolute path to the second directory
    pub right: String,
}

#[tool_router(router = comparing_tools, vis = "pub(super)")]
impl FfmServer {
    /// What two directories disagree about.
    #[tool(
        description = "Compare two directories: which files are only on the left, only on the right, and which exist in both but differ in size. Compares names and sizes across the trees, not content."
    )]
    pub(crate) fn compare_directories(
        &self,
        Parameters(params): Parameters<CompareParams>,
    ) -> Result<CallToolResult, McpError> {
        let left_root = self.safe_dir(&params.left)?;
        let right_root = self.safe_dir(&params.right)?;

        let left = directory_sizes(&left_root);
        let right = directory_sizes(&right_root);

        let only_left: Vec<&String> = left
            .keys()
            .filter(|key| !right.contains_key(*key))
            .collect();
        let only_right: Vec<&String> = right
            .keys()
            .filter(|key| !left.contains_key(*key))
            .collect();
        let differing: Vec<serde_json::Value> = left
            .iter()
            .filter_map(|(name, size)| {
                let other = right.get(name)?;
                (other != size)
                    .then(|| json!({ "path": name, "left_size": size, "right_size": other }))
            })
            .collect();

        let result = json!({
            "left": left_root.display().to_string(),
            "right": right_root.display().to_string(),
            "identical": only_left.is_empty() && only_right.is_empty() && differing.is_empty(),
            "only_in_left": only_left,
            "only_in_right": only_right,
            "differing_size": differing,
        });
        Ok(json_response(&result))
    }

    /// Whether two files are the same, and where they first stop being so.
    #[tool(
        description = "Compare two files byte for byte. Reports whether they are identical, their sizes and SHA-256 sums, and the offset of the first difference. Different sizes are answered without hashing."
    )]
    pub(crate) fn compare_files(
        &self,
        Parameters(params): Parameters<CompareFilesParams>,
    ) -> Result<CallToolResult, McpError> {
        let left = self.safe_file(&params.left)?;
        let right = self.safe_file(&params.right)?;

        // A size difference settles "identical" without hashing — but not
        // where the difference is. "short" and "much longer content" disagree
        // at byte 0, so the offset is read from the bytes, never guessed from
        // the sizes. Equal-size files fold SHA-256 into the same single read.
        let comparison = compare_file_pair(&left, &right).map_err(crate::tools::io_error)?;

        let mut result = json!({
            "left": left.display().to_string(),
            "right": right.display().to_string(),
            "left_size": comparison.left_size,
            "right_size": comparison.right_size,
            "identical": matches!(comparison.outcome, ComparisonOutcome::Identical(_)),
        });
        record_outcome(&mut result, comparison.outcome);
        Ok(json_response(&result))
    }
}

fn compare_file_pair(left: &Path, right: &Path) -> io::Result<FilePairComparison> {
    let left_file = fs::File::open(left)?;
    let right_file = fs::File::open(right)?;
    let left_size = left_file.metadata()?.len();
    let right_size = right_file.metadata()?.len();
    let outcome = compare_streams(left_file, right_file, left_size, right_size)?;
    Ok(FilePairComparison {
        left_size,
        right_size,
        outcome,
    })
}

fn record_outcome(result: &mut serde_json::Value, outcome: ComparisonOutcome) {
    match outcome {
        ComparisonOutcome::Identical(sha256) => {
            result["sha256"] = json!(sha256);
        }
        ComparisonOutcome::Different { offset, digests } => {
            result["first_difference_at"] = json!(offset);
            if let Some(digests) = digests {
                result["left_sha256"] = json!(digests.left_sha256);
                result["right_sha256"] = json!(digests.right_sha256);
            }
        }
    }
}

fn directory_sizes(root: &Path) -> BTreeMap<String, u64> {
    let mut files = Vec::new();
    collect_files(root, root, &mut files, WALK_BUDGET);
    files
        .into_iter()
        .map(|file| (file.relative, file.size))
        .collect::<BTreeMap<_, _>>()
}
