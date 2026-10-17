//! Purpose: `wc` for one text file: lines, words, characters, bytes and the
//! longest line, streamed.

use super::{json_response, PathParam};
use crate::conversions::saturating_u64;
use crate::FfmServer;
use rmcp::{
    handler::server::wrapper::Parameters, model::CallToolResult, tool, tool_router,
    ErrorData as McpError,
};
use serde_json::json;
use std::fs;
use std::io;
use std::path::Path;

#[tool_router(router = file_stats_tools, vis = "pub(super)")]
impl FfmServer {
    /// `wc`, for a caller that wants the shape of a file before opening it.
    #[tool(
        description = "Count a text file's lines, words, characters and bytes, and report its longest line. Streams the file rather than loading it."
    )]
    pub(crate) fn file_stats(
        &self,
        Parameters(params): Parameters<PathParam>,
    ) -> Result<CallToolResult, McpError> {
        const MAX_STATS_BYTES: u64 = 100 * 1024 * 1024;
        let path = self.safe_file(&params.path)?;
        let on_disk = fs::metadata(&path)
            .map_err(|e| McpError::internal_error(e.to_string(), None))?
            .len();
        if on_disk > MAX_STATS_BYTES {
            return Err(McpError::invalid_params(
                format!("file exceeds maximum size of {MAX_STATS_BYTES} bytes"),
                None,
            ));
        }
        let counts = count_file_text(&path)?;

        let result = json!({
            "path": path.display().to_string(),
            "lines": counts.lines,
            "words": counts.words,
            "characters": counts.chars,
            // The streamed count assumes a newline after every line; the file's
            // own length is the truth, and the two differ by one on a file that
            // does not end with a newline.
            "bytes": on_disk,
            "bytes_counted": counts.bytes,
            "longest_line_chars": counts.longest,
            "longest_line_number": counts.longest_at,
        });
        Ok(json_response(&result))
    }
}

struct TextCounts {
    lines: u64,
    words: u64,
    chars: u64,
    bytes: u64,
    longest: usize,
    longest_at: u64,
}

fn count_file_text(path: &Path) -> Result<TextCounts, McpError> {
    let file = fs::File::open(path).map_err(|e| McpError::internal_error(e.to_string(), None))?;
    let mut lines = 0u64;
    let mut words = 0u64;
    let mut chars = 0u64;
    let mut bytes = 0u64;
    let mut longest = 0usize;
    let mut longest_at = 0u64;

    for (number, line) in io::BufRead::split(io::BufReader::new(file), b'\n').enumerate() {
        let line = line.map_err(|e| McpError::internal_error(e.to_string(), None))?;
        let text = String::from_utf8_lossy(&line);
        let char_count = text.chars().count();
        lines += 1;
        words += saturating_u64(text.split_whitespace().count());
        chars += saturating_u64(char_count);
        // `split` dropped the separator; every line but a last one without
        // a trailing newline had one.
        bytes += saturating_u64(line.len()) + 1;
        if char_count > longest {
            longest = char_count;
            longest_at = saturating_u64(number) + 1;
        }
    }
    Ok(TextCounts {
        lines,
        words,
        chars,
        bytes,
        longest,
        longest_at,
    })
}
