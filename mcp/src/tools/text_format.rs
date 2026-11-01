//! Purpose: How a text file is punctuated — line endings, final newline,
//! byte-order mark — which an edit has to preserve.

use super::{json_response, PathParam};
use crate::conversions::{count_byte, saturating_u64};
use crate::FfmServer;
use rmcp::{
    handler::server::wrapper::Parameters, model::CallToolResult, tool, tool_router,
    ErrorData as McpError,
};
use serde_json::json;
use std::fs;
use std::io::{self, Read, Seek, SeekFrom};
use std::path::Path;

/// How much of a file `text_format` looks at. The punctuation of a text file is
/// established in its first pages; reading a gigabyte to confirm it would make
/// the cheap question expensive.
pub const MAX_FORMAT_BYTES: usize = 256 * 1024;

/// At most `limit` bytes from the front of `path`.
fn read_capped(path: &Path, limit: usize) -> io::Result<Vec<u8>> {
    let mut buffer = Vec::new();
    fs::File::open(path)?
        .take(saturating_u64(limit))
        .read_to_end(&mut buffer)?;
    Ok(buffer)
}

/// The last byte of `path`, or `None` for an empty file. One seek and one
/// read: the end of a file is never worth streaming to.
fn last_byte(path: &Path) -> io::Result<Option<u8>> {
    let mut file = fs::File::open(path)?;
    if file.seek(SeekFrom::End(0))? == 0 {
        return Ok(None);
    }
    file.seek(SeekFrom::End(-1))?;
    let mut byte = [0u8; 1];
    file.read_exact(&mut byte)?;
    Ok(Some(byte[0]))
}

/// What a text file's punctuation is, which an edit has to preserve.
struct TextFormat {
    endings: &'static str,
    crlf: usize,
    lf: usize,
    cr: usize,
    final_newline: bool,
    bom: &'static str,
    binary: bool,
}

impl TextFormat {
    pub(crate) fn of(bytes: &[u8]) -> Self {
        let bom = match bytes {
            [0xEF, 0xBB, 0xBF, ..] => "utf-8",
            [0xFF, 0xFE, ..] => "utf-16le",
            [0xFE, 0xFF, ..] => "utf-16be",
            _ => "none",
        };
        // A CR is only a lone CR when no LF follows it, which is the same rule
        // that makes a CRLF one ending rather than two.
        let crlf = bytes.windows(2).filter(|pair| pair == b"\r\n").count();
        let lf = count_byte(bytes, b'\n') - crlf;
        let cr = count_byte(bytes, b'\r') - crlf;
        let endings = match (crlf > 0, lf > 0, cr > 0) {
            (true, false, false) => "crlf",
            (false, true, false) => "lf",
            (false, false, true) => "cr",
            (false, false, false) => "none",
            _ => "mixed",
        };
        Self {
            endings,
            crlf,
            lf,
            cr,
            final_newline: bytes.last() == Some(&b'\n'),
            bom,
            // A NUL byte is what every editor uses to decide the same thing:
            // no text encoding this server can report puts one in the middle.
            binary: bytes.contains(&0),
        }
    }
}

#[tool_router(router = text_format_tools, vis = "pub(super)")]
impl FfmServer {
    /// How a text file is punctuated, which is what an editor has to preserve.
    #[tool(
        description = "Report a text file's line endings (lf/crlf/cr/mixed), whether it ends with a newline, its byte-order mark, and whether it looks binary. Endings are judged from the first 256 KB (see bytes_examined); the final newline is read from the true end of the file. Read before editing a file you did not write."
    )]
    pub(crate) fn text_format(
        &self,
        Parameters(params): Parameters<PathParam>,
    ) -> Result<CallToolResult, McpError> {
        let path = self.safe_file(&params.path)?;
        // One byte past the cap decides a boundary CR: followed by LF it is
        // half of a CRLF pair (counting the orphan would flip a clean CRLF
        // file to "mixed"); followed by anything else it is a genuine lone CR
        // that must stay in the count.
        let mut bytes = read_capped(&path, MAX_FORMAT_BYTES + 1)
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;
        let sampled = bytes.len() > MAX_FORMAT_BYTES;
        if sampled {
            let past_cap = bytes.pop();
            if bytes.last() == Some(&b'\r') && past_cap == Some(b'\n') {
                bytes.pop();
            }
        }
        let mut format = TextFormat::of(&bytes);
        // The final newline is a property of the file's last byte, which the
        // sample never reaches on a file larger than the cap.
        if sampled {
            format.final_newline = last_byte(&path)
                .map_err(|e| McpError::internal_error(e.to_string(), None))?
                == Some(b'\n');
        }

        let result = json!({
            "path": path.display().to_string(),
            "line_endings": format.endings,
            "crlf_count": format.crlf,
            "lf_count": format.lf,
            "cr_count": format.cr,
            "final_newline": format.final_newline,
            "byte_order_mark": format.bom,
            "binary": format.binary,
            "bytes_examined": bytes.len(),
        });
        Ok(json_response(&result))
    }
}
