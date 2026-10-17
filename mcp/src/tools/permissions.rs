//! Purpose: Reading and setting a file's permissions: a Unix mode or the
//! cross-platform read-only flag.

use super::json_response;
use crate::{fsops, FfmServer};
use rmcp::{
    handler::server::wrapper::Parameters, model::CallToolResult, tool, tool_router,
    ErrorData as McpError,
};
use serde::Deserialize;
use serde_json::json;
use std::fs;
use std::path::Path;

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct PermissionsParams {
    /// Path to the file or directory
    pub path: String,
    /// Unix permission mode as octal string (e.g. "755", "644")
    #[serde(default)]
    pub mode: Option<String>,
    /// Set read-only flag (cross-platform)
    #[serde(default)]
    pub readonly: Option<bool>,
}

#[tool_router(router = permission_tools, vis = "pub(super)")]
impl FfmServer {
    /// Get or set file permissions.
    #[tool(
        description = "Get or set file permissions. Provide 'mode' (Unix octal like '755') or 'readonly' (bool) to set."
    )]
    pub(crate) fn permissions(
        &self,
        Parameters(params): Parameters<PermissionsParams>,
    ) -> Result<CallToolResult, McpError> {
        if params.mode.is_some() || params.readonly.is_some() {
            self.require_readwrite()?;
        }
        let path = self.safe_path(&params.path)?;

        // `readonly` runs first so that a caller supplying both gets exactly
        // the `mode` they asked for instead of a mode widened afterwards.
        if let Some(readonly) = params.readonly {
            fsops::apply_readonly(&path, readonly)
                .map_err(|e| McpError::internal_error(e.to_string(), None))?;
        }

        #[cfg(unix)]
        if let Some(mode_str) = &params.mode {
            use std::os::unix::fs::PermissionsExt;
            let mode = u32::from_str_radix(mode_str, 8)
                .map_err(|e| McpError::invalid_params(format!("invalid octal mode: {e}"), None))?;
            if mode > 0o7777 {
                return Err(McpError::invalid_params(
                    format!("mode {mode_str} exceeds maximum 7777 octal"),
                    None,
                ));
            }
            fs::set_permissions(&path, fs::Permissions::from_mode(mode))
                .map_err(|e| McpError::internal_error(e.to_string(), None))?;
        }
        #[cfg(not(unix))]
        // `mode` remains in the cross-platform tool schema because MCP clients
        // can connect to either host. It is explicitly Unix-only and retains
        // its existing no-op behaviour elsewhere.
        let _ignored_unix_mode = params.mode.as_deref();

        let result = permissions_report(&path)?;

        Ok(json_response(&result))
    }
}

fn permissions_report(path: &Path) -> Result<serde_json::Value, McpError> {
    // Re-read so the response reports what is actually on disk.
    let perms = fs::metadata(path)
        .map_err(|e| McpError::internal_error(e.to_string(), None))?
        .permissions();

    #[cfg(unix)]
    let reported_mode = {
        use std::os::unix::fs::PermissionsExt;
        Some(format!("{:o}", perms.mode()))
    };
    #[cfg(not(unix))]
    let reported_mode: Option<String> = None;

    let mut result = serde_json::Map::new();
    result.insert("path".to_owned(), json!(path.display().to_string()));
    result.insert("readonly".to_owned(), json!(perms.readonly()));
    if let Some(mode) = reported_mode {
        result.insert("mode".to_owned(), json!(mode));
    }

    Ok(serde_json::Value::Object(result))
}
