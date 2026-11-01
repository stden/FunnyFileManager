//! FFM MCP Server — Model Context Protocol server for `FunnyFileManager`.
//!
//! Exposes file management operations as MCP tools so that AI assistants
//! (Claude, `ChatGPT`, etc.) can manage files, directories, and S3 objects.
//!
//! Built on the official `rmcp` SDK with stdio transport. This file composes
//! the server: file-tool confinement, application-session authority, handler and startup.
//! The tools themselves are declared per area under `tools/`.

use rmcp::{
    model::{ServerCapabilities, ServerInfo},
    tool_handler,
    transport::stdio,
    ErrorData as McpError, ServerHandler, ServiceExt,
};
use std::io;
use std::path::PathBuf;
use tracing::info;

mod automation;
mod checksum;
mod cleanup;
mod content_search;
mod conversions;
mod directory_tools;
mod fsops;
mod globmatch;
mod guards;
mod kdbx;
mod naming;
mod paths;
mod pattern_search;
mod replace;
mod reports;
mod s3_listing;
mod s3_profiles;
// The tool list lives in its own import-free file so the sibling crate's MCP
// status screen can `include!` it instead of keeping a second copy.
mod server;
mod tool_names;
mod tools;
mod walk;
pub use server::FfmServer;
pub use tool_names::TOOL_NAMES;

// What the tool modules beside this file import from the crate root, where
// these items lived before the tools moved out.
use checksum::checksum_of;
use conversions::{format_size, saturating_u64};
use tools::analysis::{AnalyseParams, DuplicatesParams};
use tools::listing::SortDirectoryParams;
use tools::patterns::{DeleteByPatternParams, GlobParams, ReplaceParams};
use tools::search::GrepParams;
use tools::{json_response, system::dirs_home};
use walk::{
    collect_files, content_kind, relative_slash_path, walkable_type, CollectedFile, WALK_BUDGET,
};

#[cfg(test)]
mod tests;
#[cfg(test)]
mod tests_conversions;
#[cfg(test)]
mod tests_guards;
#[cfg(test)]
mod tests_kdbx;
#[cfg(test)]
mod tests_tools;

impl FfmServer {
    /// Fails with a read-only error when the server was started in readonly mode.
    fn require_readwrite(&self) -> Result<(), McpError> {
        guards::require_readwrite(self.readonly)
    }

    /// Fails with a disabled error when the kbdx tool is not enabled.
    fn require_kdbx_allowed(&self) -> Result<(), McpError> {
        guards::require_kdbx_allowed(self.kdbx_allowed)
    }

    /// Resolve an existing caller-supplied path, confined to the server root.
    fn safe_path(&self, raw: &str) -> Result<PathBuf, McpError> {
        paths::sanitize_path(&self.root, raw).map_err(|e| McpError::invalid_params(e, None))
    }

    fn safe_dir(&self, raw: &str) -> Result<PathBuf, McpError> {
        let path = self.safe_path(raw)?;
        if !path.is_dir() {
            return Err(McpError::invalid_params(
                format!("not a directory: {}", path.display()),
                None,
            ));
        }
        Ok(path)
    }

    fn safe_file(&self, raw: &str) -> Result<PathBuf, McpError> {
        let path = self.safe_path(raw)?;
        if !path.is_file() {
            return Err(McpError::invalid_params(
                format!("not a file: {}", path.display()),
                None,
            ));
        }
        Ok(path)
    }

    /// Same, for paths that do not exist yet (create/write destinations).
    fn safe_path_create(&self, raw: &str) -> Result<PathBuf, McpError> {
        paths::sanitize_path_create(&self.root, raw).map_err(|e| McpError::invalid_params(e, None))
    }

    /// Same, but keeps a final symlink unresolved — for operations that must
    /// act on the link itself rather than on its target.
    fn safe_path_nofollow(&self, raw: &str) -> Result<PathBuf, McpError> {
        paths::sanitize_path_nofollow(&self.root, raw)
            .map_err(|e| McpError::invalid_params(e, None))
    }

    /// Same as `safe_path_nofollow`, for one entry of a batch: the reason an
    /// entry is refused is reported beside it instead of failing the call.
    fn safe_path_nofollow_entry(&self, raw: &str) -> Result<PathBuf, String> {
        paths::sanitize_path_nofollow(&self.root, raw)
    }
}

// Dispatch through the router built in `new()` — the default
// `Self::tool_router()` would rebuild it on every call and leaves the field
// unread (a `dead_code` warning that `clippy -D warnings` turns into an error).
#[tool_handler(router = self.tool_router)]
#[expect(
    clippy::unused_async_trait_impl,
    reason = "rmcp's tool_handler macro generates the async dispatch methods"
)]
impl ServerHandler for FfmServer {
    fn get_info(&self) -> ServerInfo {
        ServerInfo::new(ServerCapabilities::builder().enable_tools().build())
            .with_instructions(
                "FunnyFileManager MCP server — comprehensive file management tools for AI assistants. \
                 Supports local filesystem operations (list, view, create, copy, move, delete, search, grep, tree), \
                 pattern matching (glob, find with filters, replace in files, delete by pattern), \
                 batch operations, bookmarks, symlinks, permissions, checksums, \
                 read-only KeePass export to JSON, YAML, or dotenv, \
                 S3 cloud storage (list profiles/buckets/objects, upload, download, delete, copy), \
                 and system information. File tools are confined to the allowed root. \
                 The separately enabled ffm_session_* tools control private headless FFM sessions \
                 with the user's full application authority, including shell, configuration, \
                 WSL and remote servers. They require FFM_MCP_ALLOW_APPLICATION=1 and a writable server; \
                 sessions can navigate outside the file-tool root."
                    .to_owned(),
            )
    }
}

// ─── Main ───────────────────────────────────────────────────────

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Log to stderr only — stdout is reserved for JSON-RPC
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive(tracing::Level::INFO.into()),
        )
        .with_writer(io::stderr)
        .with_ansi(false)
        .init();

    // Resolved once: file tools and initial session paths use this allowed tree.
    let root: PathBuf = paths::resolve_root()?;
    info!(root = %root.display(), tools = TOOL_NAMES.len(), "ffm-mcp server starting");

    FfmServer::new(root).serve(stdio()).await?.waiting().await?;

    info!("ffm-mcp server stopped");
    Ok(())
}
