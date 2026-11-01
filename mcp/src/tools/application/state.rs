//! Purpose: Observe the action catalog, state and bounded background-job progress.

use super::params::{SessionParams, WaitParams};
use crate::{automation::Request, FfmServer};
use rmcp::{
    handler::server::wrapper::Parameters, model::CallToolResult, tool, tool_router,
    ErrorData as McpError,
};

#[tool_router(router = application_state_tools, vis = "pub(super)")]
impl FfmServer {
    #[tool(
        description = "List canonical FFM contexts/actions, descriptions and default key bindings. Requires an explicitly enabled application session.",
        annotations(read_only_hint = true, open_world_hint = true)
    )]
    pub(crate) async fn ffm_session_catalog(
        &self,
        Parameters(params): Parameters<SessionParams>,
    ) -> Result<CallToolResult, McpError> {
        self.application_request(&params.session_id, Request::Catalog)
            .await
    }

    #[tool(
        description = "Read application state, rendered screen and background-job status for an enabled FFM session.",
        annotations(read_only_hint = true, open_world_hint = true)
    )]
    pub(crate) async fn ffm_session_state(
        &self,
        Parameters(params): Parameters<SessionParams>,
    ) -> Result<CallToolResult, McpError> {
        self.application_request(&params.session_id, Request::State)
            .await
    }

    #[tool(
        description = "Wait up to 10000 milliseconds for an FFM background operation and return current state. Other sessions stay available.",
        annotations(read_only_hint = true, open_world_hint = true)
    )]
    pub(crate) async fn ffm_session_wait(
        &self,
        Parameters(params): Parameters<WaitParams>,
    ) -> Result<CallToolResult, McpError> {
        self.require_application()?;
        if params.timeout_ms > 10_000 {
            return Err(McpError::invalid_params(
                "timeout_ms must be at most 10000",
                None,
            ));
        }
        self.application_request(
            &params.session_id,
            Request::Wait {
                timeout_ms: params.timeout_ms,
            },
        )
        .await
    }
}
