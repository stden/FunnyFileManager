//! Purpose: Small utilities: drives, total size, a free file name, a checksum,
//! and a byte count made human-readable.

use super::{json_response, PathParam};
use crate::{checksum::checksum_file, directory_tools, naming::unique_destination};
use crate::{conversions::format_size, FfmServer};
use ffm_mcp::tree_walk::directory_size;
use rmcp::{
    handler::server::wrapper::Parameters, model::CallToolResult, tool, tool_router,
    ErrorData as McpError,
};
use serde::Deserialize;
use serde_json::json;
use std::fs;

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct UniqueNameParams {
    /// Directory where the file will be created
    pub directory: String,
    /// Desired filename (e.g. "report.txt")
    pub filename: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct FormatSizeParams {
    /// Number of bytes to format
    pub bytes: u64,
}

#[tool_router(router = utility_tools, vis = "pub(super)")]
impl FfmServer {
    /// List available drives/mount points and S3 profiles.
    #[tool(description = "List available drives, mount points, and S3 profiles.")]
    pub(crate) fn list_drives() -> CallToolResult {
        directory_tools::list_drives()
    }

    /// Calculate total size of a file or directory tree.
    #[tool(description = "Calculate total size of a file or directory tree.")]
    pub(crate) fn disk_usage(
        &self,
        Parameters(params): Parameters<PathParam>,
    ) -> Result<CallToolResult, McpError> {
        let path = self.safe_path(&params.path)?;

        if !path.exists() {
            return Err(McpError::invalid_params(
                format!("not found: {}", path.display()),
                None,
            ));
        }

        let size = directory_size(&path);
        let result = json!({
            "path": path.display().to_string(),
            "size": size,
            "size_human": format_size(size),
        });
        Ok(json_response(&result))
    }

    /// Generate a unique filename that doesn't conflict with existing files.
    #[tool(
        description = "Generate a unique filename avoiding collisions. If 'report.txt' exists, returns 'report_1.txt', etc."
    )]
    pub(crate) fn unique_name(
        &self,
        Parameters(params): Parameters<UniqueNameParams>,
    ) -> Result<CallToolResult, McpError> {
        let dir = self.safe_path(&params.directory)?;
        if !dir.is_dir() {
            return Err(McpError::invalid_params(
                format!("not a directory: {}", dir.display()),
                None,
            ));
        }

        // `invalid_params`, not `internal_error`: both failures — a name that is
        // not a plain file name, and a name whose counters are all taken — are
        // properties of the caller's argument, not of the server.
        let dest = unique_destination(&dir, &params.filename)
            .map_err(|e| McpError::invalid_params(e, None))?;

        let result = json!({
            "directory": dir.display().to_string(),
            "requested": params.filename,
            "unique_path": dest.display().to_string(),
            "unique_name": dest.file_name().unwrap_or_default().to_string_lossy(),
        });
        Ok(json_response(&result))
    }

    /// Compute SHA-256 checksum of a file.
    #[tool(description = "Compute SHA-256 checksum of a file.")]
    pub(crate) fn file_checksum(
        &self,
        Parameters(params): Parameters<PathParam>,
    ) -> Result<CallToolResult, McpError> {
        let path = self.safe_file(&params.path)?;

        let hash =
            checksum_file(&path).map_err(|e| McpError::internal_error(e.to_string(), None))?;

        let meta =
            fs::metadata(&path).map_err(|e| McpError::internal_error(e.to_string(), None))?;

        let result = json!({
            "path": path.display().to_string(),
            "sha256": hash,
            "size": meta.len(),
            "size_human": format_size(meta.len()),
        });
        Ok(json_response(&result))
    }

    /// Get human-readable size format for a byte count.
    #[tool(description = "Convert a byte count to human-readable format (e.g. 1536 → '1.50 KB').")]
    pub(crate) fn format_size_tool(
        Parameters(params): Parameters<FormatSizeParams>,
    ) -> CallToolResult {
        let result = json!({
            "bytes": params.bytes,
            "formatted": format_size(params.bytes),
        });
        json_response(&result)
    }
}
