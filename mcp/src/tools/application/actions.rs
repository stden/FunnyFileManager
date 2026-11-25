//! Purpose: Forward canonical actions, input, resize and cancellation to FFM.

use super::{
    helpers::{automation_error, validate_input},
    params::{ActionParams, InputParams, ResizeParams, SessionParams},
};
use crate::{
    automation::{Request, Size},
    FfmServer,
};
use rmcp::{
    handler::server::wrapper::Parameters, model::CallToolResult, tool, tool_router,
    ErrorData as McpError,
};

#[tool_router(router = application_action_tools, vis = "pub(super)")]
impl FfmServer {
    #[tool(
        description = "Execute an action named by ffm_session_catalog through FFM's canonical application dispatch. May run shell commands, change files/configuration, or contact remote services. Long operations return a job identifier; use state/wait.",
        annotations(
            read_only_hint = false,
            destructive_hint = true,
            open_world_hint = true
        )
    )]
    pub(crate) async fn ffm_session_action(
        &self,
        Parameters(params): Parameters<ActionParams>,
    ) -> Result<CallToolResult, McpError> {
        self.application_request(
            &params.session_id,
            Request::Action {
                context: params.context,
                action: params.action,
            },
        )
        .await
    }

    #[tool(
        description = "Send keys, pasted text or a mouse event to the current FFM screen. Input follows normal modal handling and reaches a running child terminal. Has full application authority.",
        annotations(
            read_only_hint = false,
            destructive_hint = true,
            open_world_hint = true
        )
    )]
    pub(crate) async fn ffm_session_input(
        &self,
        Parameters(params): Parameters<InputParams>,
    ) -> Result<CallToolResult, McpError> {
        self.require_application()?;
        validate_input(&params)?;
        self.application_request(
            &params.session_id,
            Request::Input {
                keys: params.keys,
                text: params.text,
                mouse: params.mouse,
            },
        )
        .await
    }

    #[tool(
        description = "Resize an FFM session's virtual terminal (width 20..400, height 8..200).",
        annotations(
            read_only_hint = false,
            destructive_hint = false,
            open_world_hint = true
        )
    )]
    pub(crate) async fn ffm_session_resize(
        &self,
        Parameters(params): Parameters<ResizeParams>,
    ) -> Result<CallToolResult, McpError> {
        self.require_application()?;
        Size::new(params.width, params.height).map_err(automation_error)?;
        self.application_request(
            &params.session_id,
            Request::Resize {
                width: params.width,
                height: params.height,
            },
        )
        .await
    }

    #[tool(
        description = "Request cancellation of the active FFM operation or running child terminal. Returns current session state.",
        annotations(
            read_only_hint = false,
            destructive_hint = false,
            open_world_hint = true
        )
    )]
    pub(crate) async fn ffm_session_cancel(
        &self,
        Parameters(params): Parameters<SessionParams>,
    ) -> Result<CallToolResult, McpError> {
        self.application_request(&params.session_id, Request::Cancel)
            .await
    }
}
