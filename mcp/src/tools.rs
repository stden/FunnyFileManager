//! Purpose: The MCP tool surface. Each area declares its tools in its own
//! `#[tool_router]`; the server's router is all of them merged.
//!
//! NOTE: there is deliberately no "open editor" tool. This server speaks
//! JSON-RPC over fd 0/1, so any child process it spawns inherits the
//! protocol streams — an interactive editor writes terminal escapes into
//! the response frames, and running a caller-named binary is arbitrary
//! code execution from a tool argument.

use crate::FfmServer;
use rmcp::{
    handler::server::router::tool::ToolRouter,
    model::{CallToolResult, ContentBlock},
};
use serde::Deserialize;

pub mod analysis;
pub mod batch;
pub mod bookmarks;
pub mod comparing;
pub mod file_stats;
pub mod keepass;
pub mod listing;
pub mod patterns;
pub mod permissions;
pub mod reading;
pub mod s3;
pub mod s3_transfer;
pub mod search;
pub mod symlinks;
pub mod system;
pub mod tail;
pub mod text_format;
pub mod transfer;
pub mod utilities;
pub mod writing;

impl FfmServer {
    /// Every tool the server offers: the routers of all areas, merged. The
    /// merged router lists its tools by name, so the areas' order is moot.
    pub(crate) fn tool_router() -> ToolRouter<Self> {
        Self::listing_tools()
            + Self::reading_tools()
            + Self::keepass_tools()
            + Self::writing_tools()
            + Self::transfer_tools()
            + Self::search_tools()
            + Self::pattern_tools()
            + Self::utility_tools()
            + Self::batch_tools()
            + Self::symlink_tools()
            + Self::permission_tools()
            + Self::bookmark_tools()
            + Self::system_tools()
            + Self::analysis_tools()
            + Self::tail_tools()
            + Self::comparing_tools()
            + Self::text_format_tools()
            + Self::file_stats_tools()
            + Self::s3_tools()
            + Self::s3_transfer_tools()
    }
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct PathParam {
    /// Absolute path to a file or directory
    pub path: String,
}

fn default_true() -> bool {
    true
}

fn default_max_results() -> usize {
    100
}

pub fn json_response(value: &serde_json::Value) -> CallToolResult {
    CallToolResult::success(vec![ContentBlock::text(
        serde_json::to_string_pretty(value).unwrap_or_default(),
    )])
}
