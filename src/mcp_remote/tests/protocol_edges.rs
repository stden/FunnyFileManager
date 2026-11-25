//! Purpose: Protocol edge cases, schema fallback, and config merge error handling tests.

use serde_json::json;

use super::super::config;
use super::super::protocol::{decode_call_result, decode_tools, parse_rpc_payload};
use super::super::schema::default_args_template;
use crate::values::mcp_remote::{HttpMcpServer, McpRemoteError};

#[test]
fn protocol_rejects_bad_json_sse_edge_events_and_missing_tools() {
    assert!(matches!(
        parse_rpc_payload("application/json", "{bad"),
        Err(McpRemoteError::Protocol(_))
    ));
    let sse = "event: ping\n\ndata: not-json\n\ndata: {\"jsonrpc\":\"2.0\",\"method\":\"notify\"}\n\n\ndata: {\"jsonrpc\":\"2.0\",\"result\":{}}\n\n";
    assert_eq!(
        parse_rpc_payload("text/event-stream", sse).expect("sse"),
        json!({})
    );
    assert!(parse_rpc_payload("text/event-stream", "data: keepalive\n\n").is_err());
    assert!(matches!(
        decode_tools(&json!({}), &[]),
        Err(McpRemoteError::Protocol(_))
    ));
}

#[test]
fn call_result_and_schema_handle_non_text_blocks_and_empty_schema() {
    let call = decode_call_result(
        "svc",
        "img",
        &json!({"content": [{"type": "image", "data": "abc"}]}),
    );
    assert!(call.text.contains("image") && call.text.contains("abc"));
    assert_eq!(default_args_template(&json!({})), "{}");
}

#[test]
fn config_discover_ignores_invalid_json_file_and_disabled_yaml_server() {
    let temp = tempfile::TempDir::new().expect("temp");
    let bad_json = temp.path().join("bad.json");
    let yml = temp.path().join("ffm.yml");
    std::fs::write(&bad_json, "{not json").expect("bad json");
    std::fs::write(
        &yml,
        "mcp_servers:\n  disabled-svc:\n    type: http\n    url: https://mcp.example.test/x\n    disabled: true\n",
    )
    .expect("yml");
    let found = config::discover(&[bad_json], Some(&yml), None, None);
    assert_eq!(found, Vec::<HttpMcpServer>::new());
}
