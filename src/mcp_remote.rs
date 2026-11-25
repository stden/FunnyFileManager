//! Purpose: Streamable HTTP MCP client adapter loading Claude Code / Kiro / `ffm.yml` services.

pub mod client;
pub mod config;
pub mod protocol;
pub mod schema;
pub mod session;

use crate::values::mcp_remote::{McpRemoteError, McpReply, McpRequest};
use client::{HttpRpcTransport, RpcTransport};
use session::{call_tool_with, list_tools_with};

/// Execute one HTTP MCP request on a background worker thread.
#[must_use]
pub fn execute(request: McpRequest) -> McpReply {
    let server = match &request {
        McpRequest::ListTools(server) | McpRequest::CallTool { server, .. } => server,
    };
    match HttpRpcTransport::from_server(server) {
        Ok(mut transport) => execute_with(&mut transport, request),
        Err(error) => McpReply::fail(&request, error),
    }
}

/// Execute one HTTP MCP request over `transport`.
pub fn execute_with(transport: &mut dyn RpcTransport, request: McpRequest) -> McpReply {
    match request {
        McpRequest::ListTools(server) => McpReply::Tools(
            server.name,
            list_tools_with(transport, &server.disabled_tools),
        ),
        McpRequest::CallTool {
            server,
            tool,
            arguments,
        } => McpReply::Called(call_tool_with(transport, &server.name, &tool, &arguments)),
    }
}

/// Answer a request when network access is disabled (replays / tests).
#[must_use]
pub fn unavailable(request: &McpRequest) -> McpReply {
    McpReply::fail(request, McpRemoteError::Disabled)
}

#[cfg(test)]
mod tests;
