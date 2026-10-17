//! Purpose: The last lines of a file, read backwards from its end.

use super::json_response;
use crate::conversions::count_byte;
use crate::FfmServer;
use rmcp::{
    handler::server::wrapper::Parameters, model::CallToolResult, tool, tool_router,
    ErrorData as McpError,
};
use serde::Deserialize;
use serde_json::json;
use std::fs;
use std::io::{self, Read, Seek, SeekFrom};
use std::path::Path;

/// How far `tail_file` will read backwards. A file with no newlines would
/// otherwise be slurped whole, which is the opposite of seeking from the end.
const MAX_TAIL_BYTES: usize = 10 * 1024 * 1024;

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct TailParams {
    /// Absolute path to the file
    pub path: String,
    /// How many lines from the end (default 20)
    #[serde(default)]
    pub lines: Option<usize>,
}

#[tool_router(router = tail_tools, vis = "pub(super)")]
impl FfmServer {
    /// The end of a file, without reading the front of it.
    #[tool(
        description = "Read the last N lines of a file. Seeks from the end rather than reading the whole file, so it is usable on a log of any size."
    )]
    pub(crate) fn tail_file(
        &self,
        Parameters(params): Parameters<TailParams>,
    ) -> Result<CallToolResult, McpError> {
        let path = self.safe_file(&params.path)?;
        let wanted = params.lines.unwrap_or(20);
        let (lines, total_bytes) =
            tail_lines(&path, wanted).map_err(|e| McpError::internal_error(e.to_string(), None))?;

        let result = json!({
            "path": path.display().to_string(),
            "file_size": total_bytes,
            "lines_returned": lines.len(),
            "content": lines.join("\n"),
        });
        Ok(json_response(&result))
    }
}

/// The last `wanted` lines of a file, and its size.
///
/// Reads backwards in blocks until enough newlines have gone by, so tailing a
/// gigabyte log costs the same as tailing a small one. A file smaller than one
/// block is simply read whole.
fn tail_lines(path: &Path, wanted: usize) -> io::Result<(Vec<String>, u64)> {
    let mut file = fs::File::open(path)?;
    let size = file.seek(SeekFrom::End(0))?;
    let wanted = wanted.min(10_000);
    if wanted == 0 {
        return Ok((Vec::new(), size));
    }

    let buffer = read_tail_bytes(&mut file, size, wanted)?;

    let text = String::from_utf8_lossy(&buffer);
    let mut lines: Vec<String> = text.lines().map(ToOwned::to_owned).collect();
    if lines.len() > wanted {
        lines.drain(..lines.len() - wanted);
    }
    Ok((lines, size))
}

fn read_tail_bytes(file: &mut fs::File, size: u64, wanted: usize) -> io::Result<Vec<u8>> {
    const BLOCK: usize = 8 * 1024;
    let mut chunks: Vec<Vec<u8>> = Vec::new();
    let mut total_bytes = 0usize;
    let mut position = size;
    let mut newlines = 0usize;
    // One more newline than lines wanted: the last block read may begin
    // mid-line, and that partial line is not one of the answers.
    while position > 0 && newlines <= wanted && total_bytes < MAX_TAIL_BYTES {
        let room = MAX_TAIL_BYTES - total_bytes;
        let step = u64::try_from(BLOCK.min(room))
            .unwrap_or(u64::MAX)
            .min(position);
        if step == 0 {
            break;
        }
        position -= step;
        file.seek(SeekFrom::Start(position))?;
        let mut block = vec![0u8; usize::try_from(step).unwrap_or(BLOCK)];
        file.read_exact(&mut block)?;
        newlines += count_byte(&block, b'\n');
        total_bytes += block.len();
        chunks.push(block);
    }

    let mut buffer = Vec::with_capacity(total_bytes);
    for chunk in chunks.into_iter().rev() {
        buffer.extend_from_slice(&chunk);
    }
    Ok(buffer)
}
