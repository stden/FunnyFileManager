//! Purpose: MCP initialize handshake, paginated `tools/list`, and `tools/call` orchestration.

use serde_json::Value;

use super::client::RpcTransport;
use super::protocol::{
    call_tool_body, decode_call_result, decode_tools, initialize_body, initialized_notification,
    list_tools_body, parse_rpc_payload,
};
use crate::values::mcp_remote::{McpCallResult, McpRemoteError, McpTool};

fn initialize_session(transport: &mut dyn RpcTransport) -> Result<Option<String>, McpRemoteError> {
    let init = transport.post(None, &initialize_body(1))?;
    let session_id = init.session_id.clone();
    drop(parse_rpc_payload(&init.content_type, &init.body)?);
    drop(transport.post(session_id.as_deref(), &initialized_notification()));
    Ok(session_id)
}

/// Handshake + paginated `tools/list` against an MCP endpoint.
pub fn list_tools_with(
    transport: &mut dyn RpcTransport,
    disabled_tools: &[String],
) -> Result<Vec<McpTool>, McpRemoteError> {
    let session_id = initialize_session(transport)?;
    let mut all = Vec::new();
    let mut cursor: Option<String> = None;
    let mut next_id = 2u64;
    loop {
        let resp = transport.post(
            session_id.as_deref(),
            &list_tools_body(next_id, cursor.as_deref()),
        )?;
        let result = parse_rpc_payload(&resp.content_type, &resp.body)?;
        let (batch, next) = decode_tools(&result, disabled_tools)?;
        all.extend(batch);
        cursor = next;
        if cursor.is_none() {
            break;
        }
        next_id = next_id.saturating_add(1);
    }
    all.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(all)
}

/// Handshake + `tools/call` against an MCP endpoint.
pub fn call_tool_with(
    transport: &mut dyn RpcTransport,
    server_name: &str,
    tool: &str,
    arguments: &Value,
) -> Result<McpCallResult, McpRemoteError> {
    let session_id = initialize_session(transport)?;
    let resp = transport.post(session_id.as_deref(), &call_tool_body(2, tool, arguments))?;
    let result = parse_rpc_payload(&resp.content_type, &resp.body)?;
    Ok(decode_call_result(server_name, tool, &result))
}
