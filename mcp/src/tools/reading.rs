//! Purpose: Reading a whole text file, its first lines, or a window of lines,
//! never buffering more than the read cap.

use super::json_response;
use crate::conversions::{format_size, saturating_usize};
use crate::FfmServer;
use rmcp::{
    handler::server::wrapper::Parameters, model::CallToolResult, tool, tool_router,
    ErrorData as McpError,
};
use serde::Deserialize;
use serde_json::json;
use std::fs;
use std::io::{self, Read};
use std::path::Path;

/// Bytes `read_line_range` will consume before it gives up, and the largest
/// file `view_file` accepts. Without the cap, a `read_lines` call with no `to`
/// — or a file with no newlines — would buffer without bound.
pub const MAX_READ_BYTES: u64 = 10 * 1024 * 1024;

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct HeadParams {
    /// Absolute path to the file
    pub path: String,
    /// How many lines from the start (default 20)
    #[serde(default)]
    pub lines: Option<usize>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct LineRangeParams {
    /// Absolute path to the file
    pub path: String,
    /// First line to return, 1-based and inclusive (default 1)
    #[serde(default)]
    pub from: Option<usize>,
    /// Last line to return, 1-based and inclusive (default: to the end)
    #[serde(default)]
    pub to: Option<usize>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct ViewFileParams {
    /// Absolute path to the file to view
    pub path: String,
    /// Line offset to start reading from (0-based, default 0)
    #[serde(default)]
    pub offset: Option<u64>,
    /// Maximum number of lines to return (default: all)
    #[serde(default)]
    pub limit: Option<u64>,
}

#[tool_router(router = reading_tools, vis = "pub(super)")]
impl FfmServer {
    /// Read and return the contents of a text file. Max 10 MB.
    #[tool(description = "Read and return the contents of a text file. Max 10 MB.")]
    pub(crate) fn view_file(
        &self,
        Parameters(params): Parameters<ViewFileParams>,
    ) -> Result<CallToolResult, McpError> {
        let path = self.safe_path(&params.path)?;

        if path.is_dir() {
            return Err(McpError::invalid_params("cannot view a directory", None));
        }

        let meta =
            fs::metadata(&path).map_err(|e| McpError::internal_error(e.to_string(), None))?;
        if meta.len() > MAX_READ_BYTES {
            return Err(McpError::invalid_params(
                format!("file too large: {} (max 10 MB)", format_size(meta.len())),
                None,
            ));
        }

        let content =
            fs::read_to_string(&path).map_err(|e| McpError::internal_error(e.to_string(), None))?;
        let lines: Vec<&str> = content.lines().collect();
        let total_lines = lines.len();

        let offset = params.offset.map_or(0, saturating_usize);
        let limit = params.limit.map_or(total_lines, saturating_usize);

        let selected: Vec<&str> = lines.into_iter().skip(offset).take(limit).collect();

        let result = json!({
            "path": path.display().to_string(),
            "total_lines": total_lines,
            "offset": offset,
            "lines_returned": selected.len(),
            "content": selected.join("\n"),
        });
        Ok(json_response(&result))
    }

    /// The start of a file, without reading the rest of it.
    #[tool(
        description = "Read the first N lines of a file. Stops reading once it has them, so it is usable on a file of any size; refuses cleanly if finding them takes more than 10 MB."
    )]
    pub(crate) fn head_file(
        &self,
        Parameters(params): Parameters<HeadParams>,
    ) -> Result<CallToolResult, McpError> {
        let path = self.safe_file(&params.path)?;
        let wanted = params.lines.unwrap_or(20);
        let (lines, reached_end) = read_line_range(&path, 1, wanted)
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;

        let result = json!({
            "path": path.display().to_string(),
            "lines_returned": lines.len(),
            "reached_end": reached_end,
            "content": lines.join("\n"),
        });
        Ok(json_response(&result))
    }

    /// A window of lines out of the middle of a file.
    #[tool(
        description = "Read a range of lines from a file, 1-based and inclusive. Reads only as far as 'to', so a window near the front of a huge file costs a window; refuses cleanly if the read passes 10 MB."
    )]
    pub(crate) fn read_lines(
        &self,
        Parameters(params): Parameters<LineRangeParams>,
    ) -> Result<CallToolResult, McpError> {
        let path = self.safe_file(&params.path)?;
        let from = params.from.unwrap_or(1).max(1);
        let to = params.to.unwrap_or(usize::MAX);
        if to < from {
            return Err(McpError::invalid_params(
                format!("'to' ({to}) is before 'from' ({from})"),
                None,
            ));
        }
        // `to - from` cannot overflow after the guard above.
        let (lines, reached_end) = read_line_range(&path, from, to - from + 1)
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;

        let result = json!({
            "path": path.display().to_string(),
            "from": from,
            "lines_returned": lines.len(),
            "reached_end": reached_end,
            "content": lines.join("\n"),
        });
        Ok(json_response(&result))
    }
}

/// `count` lines starting at `start` (1-based), and whether the read consumed
/// the file to its end.
///
/// Reads no further than it must: `head_file` and a window near the front of a
/// large file both stop as soon as they have what they came for. Gives up past
/// [`MAX_READ_BYTES`]: a request with no end — or a file with no newlines —
/// would otherwise buffer the whole file, which no other reader here does.
fn read_line_range(path: &Path, start: usize, count: usize) -> io::Result<(Vec<String>, bool)> {
    use io::BufRead;

    let mut lines = Vec::new();
    if count == 0 {
        return Ok((lines, false));
    }
    // One byte past the cap, so a request satisfied at the cap exactly is
    // distinguishable from one that needed more than it.
    let mut reader = io::BufReader::new(fs::File::open(path)?).take(MAX_READ_BYTES + 1);
    for (number, line) in reader.by_ref().lines().enumerate() {
        let number = number + 1;
        if number < start {
            continue;
        }
        lines.push(line?);
        if lines.len() == count {
            break;
        }
    }
    if reader.limit() == 0 {
        return Err(io::Error::other(format!(
            "gave up after reading {}: narrow the range, or use tail_file for the end",
            format_size(MAX_READ_BYTES)
        )));
    }
    // `reached_end` must mean "the file was consumed to EOF", even when the
    // count was satisfied on the file's last line — a paging client that is
    // told `false` there asks for a page past the end.
    let reached_end = reader.fill_buf()?.is_empty();
    Ok((lines, reached_end))
}
