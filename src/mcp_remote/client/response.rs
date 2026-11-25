//! Purpose: Read and validate HTTP response bodies for the Streamable HTTP MCP client.

use std::io::Read;

use reqwest::blocking::Response;
use reqwest::header::{HeaderName, CONTENT_TYPE};

use super::{RpcResponse, SESSION_HEADER};
use crate::values::mcp_remote::McpRemoteError;

const MAX_BODY: u64 = 16 * 1024 * 1024;
const ERROR_PREVIEW: usize = 200;

pub(super) fn read_rpc_response(response: Response) -> Result<RpcResponse, McpRemoteError> {
    let status = response.status().as_u16();
    let content_type = header_str(&response, CONTENT_TYPE).unwrap_or_default();
    let session_id = header_str(&response, HeaderName::from_static(SESSION_HEADER));
    read_rpc_body(status, content_type, session_id, response, MAX_BODY)
}

pub(in crate::mcp_remote) fn read_rpc_body(
    status: u16,
    content_type: String,
    session_id: Option<String>,
    body: impl Read,
    max_body: u64,
) -> Result<RpcResponse, McpRemoteError> {
    let mut bytes = Vec::new();
    body.take(max_body.saturating_add(1))
        .read_to_end(&mut bytes)
        .map_err(|err| McpRemoteError::Network(format!("Failed reading MCP response: {err}")))?;
    if !matches!(u64::try_from(bytes.len()), Ok(len) if len <= max_body) {
        return Err(McpRemoteError::Protocol(
            "MCP response exceeded size limit".into(),
        ));
    }
    let body = String::from_utf8_lossy(&bytes).into_owned();
    if !(200..300).contains(&status) {
        let preview: String = body.chars().take(ERROR_PREVIEW).collect();
        return Err(McpRemoteError::Http(status, preview));
    }
    Ok(RpcResponse {
        content_type,
        session_id,
        body,
    })
}

fn header_str(response: &Response, name: HeaderName) -> Option<String> {
    response
        .headers()
        .get(name)
        .and_then(|v| v.to_str().ok())
        .map(str::to_owned)
}
