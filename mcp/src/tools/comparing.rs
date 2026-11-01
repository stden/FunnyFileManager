//! Purpose: Comparing two directories by name and size, and two files byte
//! for byte.

use super::json_response;
use crate::checksum::checksum_file;
use crate::conversions::saturating_u64;
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
        let left_size = fs::metadata(&left)
            .map_err(|e| McpError::internal_error(e.to_string(), None))?
            .len();
        let right_size = fs::metadata(&right)
            .map_err(|e| McpError::internal_error(e.to_string(), None))?
            .len();

        // A size difference settles "identical" without hashing — but not
        // where the difference is. "short" and "much longer content" disagree
        // at byte 0, so the offset is read from the bytes, never guessed from
        // the sizes.
        let difference = first_difference(&left, &right)
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;

        let mut result = json!({
            "left": left.display().to_string(),
            "right": right.display().to_string(),
            "left_size": left_size,
            "right_size": right_size,
            "identical": difference.is_none(),
        });
        append_comparison_hashes(
            &mut result,
            &left,
            &right,
            difference,
            left_size == right_size,
        )?;
        Ok(json_response(&result))
    }
}

/// The offset of the first byte at which two files differ, or `None` when they
/// are identical. Streams both rather than loading either.
///
/// When one file is a prefix of the other, the difference is at the shorter
/// file's length — the first offset where only one of them still has a byte.
fn first_difference(left: &Path, right: &Path) -> io::Result<Option<u64>> {
    use io::BufRead;

    const BLOCK: usize = 64 * 1024;
    let mut left = io::BufReader::with_capacity(BLOCK, fs::File::open(left)?);
    let mut right = io::BufReader::with_capacity(BLOCK, fs::File::open(right)?);
    let mut offset = 0u64;

    loop {
        let left_block = left.fill_buf()?;
        let right_block = right.fill_buf()?;
        if left_block.is_empty() && right_block.is_empty() {
            return Ok(None);
        }
        if left_block.is_empty() || right_block.is_empty() {
            return Ok(Some(offset));
        }
        let step = left_block.len().min(right_block.len());
        // `zip` stops at the shorter block, so it compares exactly `step` bytes.
        if let Some(at) = left_block.iter().zip(right_block).position(|(a, b)| a != b) {
            return Ok(Some(offset + saturating_u64(at)));
        }
        left.consume(step);
        right.consume(step);
        offset += saturating_u64(step);
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

fn append_comparison_hashes(
    result: &mut serde_json::Value,
    left: &Path,
    right: &Path,
    difference: Option<u64>,
    equal_size: bool,
) -> Result<(), McpError> {
    match difference {
        None => {
            result["sha256"] =
                json!(checksum_file(left)
                    .map_err(|e| McpError::internal_error(e.to_string(), None))?);
        }
        Some(offset) => {
            result["first_difference_at"] = json!(offset);
            if equal_size {
                result["left_sha256"] = json!(checksum_file(left)
                    .map_err(|e| McpError::internal_error(e.to_string(), None))?);
                result["right_sha256"] = json!(checksum_file(right)
                    .map_err(|e| McpError::internal_error(e.to_string(), None))?);
            }
        }
    }
    Ok(())
}
