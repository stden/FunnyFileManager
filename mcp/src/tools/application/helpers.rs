//! Purpose: Enforce authority before every session lookup and bound input payloads.

use super::params::InputParams;
use crate::{
    automation::{self, Request},
    tools::json_response,
    FfmServer,
};
use rmcp::{model::CallToolResult, ErrorData as McpError};

impl FfmServer {
    pub(super) fn require_application(&self) -> Result<(), McpError> {
        self.require_readwrite()?;
        if !self.application_allowed {
            return Err(McpError::invalid_params("Application sessions are disabled. Set FFM_MCP_ALLOW_APPLICATION=1 to enable full FFM authority.", None));
        }
        Ok(())
    }

    pub(super) async fn application_request(
        &self,
        id: &automation::SessionId,
        request: Request,
    ) -> Result<CallToolResult, McpError> {
        self.require_application()?;
        let result = self
            .application
            .request(id, request)
            .await
            .map_err(automation_error)?;
        Ok(json_response(&result))
    }
}

pub(super) fn validate_input(params: &InputParams) -> Result<(), McpError> {
    if params.keys.len() > 128
        || params.keys.iter().any(|key| key.len() > 128)
        || params
            .text
            .as_ref()
            .is_some_and(|text| text.len() > 1024 * 1024)
    {
        return Err(McpError::invalid_params(
            "input exceeds 128 key chords or 1 MiB of text",
            None,
        ));
    }
    Ok(())
}

pub(super) fn automation_error(error: automation::Error) -> McpError {
    match error {
        automation::Error::Invalid(message) => McpError::invalid_params(message, None),
        automation::Error::UnknownSession => {
            McpError::invalid_params("unknown or closed FFM session", None)
        }
        automation::Error::SessionLimit => {
            McpError::invalid_params("maximum of four FFM sessions reached", None)
        }
        automation::Error::Application { code, message } => {
            McpError::invalid_params(format!("{code}: {message}"), None)
        }
        automation::Error::Io(error) => {
            McpError::internal_error(format!("FFM automation transport: {error}"), None)
        }
        automation::Error::Protocol(message) => McpError::internal_error(message, None),
        automation::Error::Timeout => {
            McpError::internal_error("FFM automation response timed out; session closed", None)
        }
    }
}
