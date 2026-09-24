//! Purpose: Creating a symbolic link and reading where one points.

use super::{json_response, PathParam};
use crate::FfmServer;
use rmcp::{
    handler::server::wrapper::Parameters, model::CallToolResult, tool, tool_router,
    ErrorData as McpError,
};
use serde::Deserialize;
use serde_json::json;
use std::fs;

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct SymlinkParams {
    /// Path where the symlink will be created
    pub link: String,
    /// Target path the symlink points to
    pub target: String,
}

#[tool_router(router = symlink_tools, vis = "pub(super)")]
impl FfmServer {
    /// Create a symbolic link.
    #[tool(description = "Create a symbolic link pointing to a target path.")]
    pub(crate) fn create_symlink(
        &self,
        Parameters(params): Parameters<SymlinkParams>,
    ) -> Result<CallToolResult, McpError> {
        let target = self.safe_path(&params.target)?;
        let link = self.safe_path_create(&params.link)?;

        #[cfg(unix)]
        {
            std::os::unix::fs::symlink(&target, &link)
                .map_err(|e| McpError::internal_error(e.to_string(), None))?;
        }

        #[cfg(windows)]
        {
            if target.is_dir() {
                std::os::windows::fs::symlink_dir(&target, &link)
                    .map_err(|e| McpError::internal_error(e.to_string(), None))?;
            } else {
                std::os::windows::fs::symlink_file(&target, &link)
                    .map_err(|e| McpError::internal_error(e.to_string(), None))?;
            }
        }

        let result = json!({
            "link": link.display().to_string(),
            "target": target.display().to_string(),
        });
        Ok(json_response(&result))
    }

    /// Read the target of a symbolic link.
    #[tool(description = "Read the target path of a symbolic link.")]
    pub(crate) fn read_symlink(
        &self,
        Parameters(params): Parameters<PathParam>,
    ) -> Result<CallToolResult, McpError> {
        // Only the parent is canonicalized — resolving the link would return
        // its target instead of the link.
        let path = self.safe_path_nofollow(&params.path)?;

        let target =
            fs::read_link(&path).map_err(|e| McpError::internal_error(e.to_string(), None))?;

        let result = json!({
            "link": path.display().to_string(),
            "target": target.display().to_string(),
            "target_exists": target.exists(),
        });
        Ok(json_response(&result))
    }
}
