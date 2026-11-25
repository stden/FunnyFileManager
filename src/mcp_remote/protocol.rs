//! Purpose: JSON-RPC 2.0 + SSE payload codec for Streamable HTTP MCP servers.

use serde_json::{json, Value};

use crate::values::mcp_remote::{McpCallResult, McpRemoteError, McpTool};

/// JSON-RPC `initialize` request payload.
#[must_use]
pub fn initialize_body(id: u64) -> Value {
    json!({
        "jsonrpc": "2.0",
        "id": id,
        "method": "initialize",
        "params": {
            "protocolVersion": "2025-03-26",
            "capabilities": {},
            "clientInfo": {
                "name": "ffm",
                "version": env!("CARGO_PKG_VERSION"),
            }
        }
    })
}

/// JSON-RPC `notifications/initialized` notification payload.
#[must_use]
pub fn initialized_notification() -> Value {
    json!({
        "jsonrpc": "2.0",
        "method": "notifications/initialized",
        "params": {}
    })
}

/// JSON-RPC `tools/list` request payload with optional pagination cursor.
#[must_use]
pub fn list_tools_body(id: u64, cursor: Option<&str>) -> Value {
    match cursor {
        Some(cursor) => json!({
            "jsonrpc": "2.0",
            "id": id,
            "method": "tools/list",
            "params": { "cursor": cursor },
        }),
        None => json!({
            "jsonrpc": "2.0",
            "id": id,
            "method": "tools/list",
            "params": {},
        }),
    }
}

/// JSON-RPC `tools/call` request payload.
#[must_use]
pub fn call_tool_body(id: u64, tool: &str, arguments: &Value) -> Value {
    json!({
        "jsonrpc": "2.0",
        "id": id,
        "method": "tools/call",
        "params": {
            "name": tool,
            "arguments": arguments,
        }
    })
}

/// Extract the JSON-RPC `result` from either `application/json` or `text/event-stream`.
pub fn parse_rpc_payload(content_type: &str, body: &str) -> Result<Value, McpRemoteError> {
    let trimmed = body.trim();
    if trimmed.is_empty() {
        return Ok(Value::Null);
    }
    let is_sse = content_type
        .to_ascii_lowercase()
        .contains("text/event-stream")
        || trimmed.starts_with("event:")
        || trimmed.starts_with("data:");
    let message = if is_sse {
        parse_sse_json(trimmed)?
    } else {
        serde_json::from_str(trimmed)
            .map_err(|err| McpRemoteError::Protocol(format!("Invalid MCP JSON: {err}")))?
    };
    if let Some(error) = message.get("error") {
        let code = error.get("code").and_then(Value::as_i64).unwrap_or(-1);
        let msg = error
            .get("message")
            .and_then(Value::as_str)
            .unwrap_or("MCP error");
        return Err(McpRemoteError::Protocol(format!("MCP error {code}: {msg}")));
    }
    Ok(message.get("result").cloned().unwrap_or(Value::Null))
}

fn parse_sse_json(body: &str) -> Result<Value, McpRemoteError> {
    let mut last_message = None;
    let mut data_lines = Vec::new();
    for line in body.lines().chain(std::iter::once("")) {
        let line = line.trim_end_matches('\r');
        if line.is_empty() {
            flush_sse_event(&mut data_lines, &mut last_message);
        } else if let Some(payload) = line.strip_prefix("data:") {
            data_lines.push(payload.trim_start().to_owned());
        }
    }
    last_message.ok_or_else(|| McpRemoteError::Protocol("No JSON-RPC result in SSE stream".into()))
}

fn flush_sse_event(data_lines: &mut Vec<String>, last_message: &mut Option<Value>) {
    if data_lines.is_empty() {
        return;
    }
    let joined = data_lines.join("\n");
    data_lines.clear();
    if let Ok(val) = serde_json::from_str::<Value>(&joined) {
        if val.get("result").is_some() || val.get("error").is_some() {
            *last_message = Some(val);
        }
    }
}

/// Decode `tools/list` response and omit `disabledTools`.
pub fn decode_tools(
    result: &Value,
    disabled_tools: &[String],
) -> Result<(Vec<McpTool>, Option<String>), McpRemoteError> {
    let Some(arr) = result.get("tools").and_then(Value::as_array) else {
        return Err(McpRemoteError::Protocol(
            "MCP tools/list missing tools array".to_owned(),
        ));
    };
    let tools = arr
        .iter()
        .filter_map(|entry| decode_tool_entry(entry, disabled_tools))
        .collect();
    let next_cursor = result
        .get("nextCursor")
        .and_then(Value::as_str)
        .filter(|s| !s.is_empty())
        .map(str::to_owned);
    Ok((tools, next_cursor))
}

fn decode_tool_entry(entry: &Value, disabled_tools: &[String]) -> Option<McpTool> {
    let name = entry.get("name").and_then(Value::as_str)?;
    if disabled_tools.iter().any(|disabled| disabled == name) {
        return None;
    }
    let description = entry
        .get("description")
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_owned();
    let input_schema = entry
        .get("inputSchema")
        .cloned()
        .unwrap_or_else(|| json!({}));
    Some(McpTool {
        name: name.to_owned(),
        description,
        input_schema,
    })
}

/// Format `tools/call` response blocks into readable text.
#[must_use]
pub fn decode_call_result(server: &str, tool: &str, result: &Value) -> McpCallResult {
    let is_error = result
        .get("isError")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let blocks = result
        .get("content")
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .map(render_content_block)
                .collect::<Vec<_>>()
                .join("\n\n")
        });
    let text = match blocks {
        Some(text) if !text.is_empty() => text,
        _ => serde_json::to_string_pretty(result).unwrap_or_else(|_| result.to_string()),
    };
    McpCallResult {
        server: server.to_owned(),
        tool: tool.to_owned(),
        is_error,
        text,
    }
}

fn render_content_block(item: &Value) -> String {
    if let Some(text) = item.get("text").and_then(Value::as_str) {
        return text.to_owned();
    }
    serde_json::to_string_pretty(item).unwrap_or_else(|_| item.to_string())
}
