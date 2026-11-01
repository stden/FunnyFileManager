//! Purpose: Start and close headless children without touching MCP protocol streams.

use super::{
    helpers::automation_error,
    params::{SessionParams, StartParams},
};
use crate::{automation::Size, tools::json_response, FfmServer};
use rmcp::{
    handler::server::wrapper::Parameters, model::CallToolResult, tool, tool_router,
    ErrorData as McpError,
};

#[tool_router(router = application_lifecycle_tools, vis = "pub(super)")]
impl FfmServer {
    #[tool(
        description = "Start a separate headless FFM application session. Requires FFM_MCP_ALLOW_APPLICATION=1 and a writable server. The session has the user's full application authority, including shell, remote servers and configuration; initial path alone is confined to the file-tool root.",
        annotations(
            read_only_hint = false,
            destructive_hint = true,
            open_world_hint = true
        )
    )]
    pub(crate) async fn ffm_session_start(
        &self,
        Parameters(params): Parameters<StartParams>,
    ) -> Result<CallToolResult, McpError> {
        self.require_application()?;
        let size = Size::new(params.width, params.height).map_err(automation_error)?;
        let root = params.path.as_deref().map_or_else(
            || self.safe_dir(&self.root.to_string_lossy()),
            |path| self.safe_dir(path),
        )?;
        let started = self
            .application
            .start(&root, size, params.profile)
            .await
            .map_err(automation_error)?;
        let result = serde_json::to_value(started)
            .map_err(|error| McpError::internal_error(error.to_string(), None))?;
        Ok(json_response(&result))
    }

    #[tool(
        description = "Close a private FFM application session and release its child process. This does not close the user's interactive FFM window.",
        annotations(
            read_only_hint = false,
            destructive_hint = true,
            open_world_hint = true
        )
    )]
    pub(crate) async fn ffm_session_close(
        &self,
        Parameters(params): Parameters<SessionParams>,
    ) -> Result<CallToolResult, McpError> {
        self.require_application()?;
        let result = self
            .application
            .close(&params.session_id)
            .await
            .map_err(automation_error)?;
        Ok(json_response(&result))
    }
}
