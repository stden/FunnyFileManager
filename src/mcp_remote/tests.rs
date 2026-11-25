//! Purpose: Unit tests for HTTP MCP discovery, env header expansion, JSON-RPC/SSE, and execution.

mod http;
mod http_errors;
mod protocol_edges;

use std::collections::BTreeMap;

use serde_json::{json, Value};

use super::client::RpcResponse;
use super::config::{
    expand_template, load_yaml_settings, parse_mcp_json, split_config_paths, YamlMcpSettings,
};
use super::protocol::{
    decode_call_result, decode_tools, initialize_body, initialized_notification, list_tools_body,
    parse_rpc_payload,
};
use super::schema::default_args_template;
use super::*;
use crate::values::mcp_remote::{HttpMcpServer, McpBusy};

struct StubRpc {
    responses: Vec<Result<RpcResponse, McpRemoteError>>,
    seen_sessions: Vec<Option<String>>,
}

impl StubRpc {
    fn new(responses: Vec<Result<RpcResponse, McpRemoteError>>) -> Self {
        Self {
            responses,
            seen_sessions: Vec::new(),
        }
    }
}

impl RpcTransport for StubRpc {
    fn post(
        &mut self,
        session_id: Option<&str>,
        _body: &Value,
    ) -> Result<RpcResponse, McpRemoteError> {
        self.seen_sessions.push(session_id.map(str::to_owned));
        self.responses.remove(0)
    }
}

fn env_map(pairs: &[(&str, &str)]) -> impl Fn(&str) -> Option<String> + use<> {
    let map: BTreeMap<String, String> = pairs
        .iter()
        .map(|&(k, v)| (k.to_owned(), v.to_owned()))
        .collect();
    move |key| map.get(key).cloned()
}

#[test]
fn env_templates_expand_and_reject_missing_or_malformed_vars() {
    let got = expand_template(
        "Token ${SVC_KEY} / ${EXTRA}",
        env_map(&[("SVC_KEY", "abc"), ("EXTRA", "xyz")]),
    )
    .expect("expanded");
    assert_eq!(got.as_str(), "Token abc / xyz");

    let missing = expand_template("${MISSING_TOKEN}", env_map(&[])).unwrap_err();
    assert!(missing.to_string().contains("MISSING_TOKEN"));
    assert!(expand_template("${UNCLOSED", env_map(&[])).is_err());
    assert!(expand_template("${}", env_map(&[])).is_err());
    assert!(expand_template("a\nb", env_map(&[])).is_err());
    assert!(expand_template("${CTL}", env_map(&[("CTL", "x\ny")])).is_err());
}

#[test]
fn discover_merges_claude_kiro_json_and_yaml_with_disabled_filters() {
    let temp = tempfile::TempDir::new().expect("temp");
    let (json_a, json_b, yml) = (
        temp.path().join("a.json"),
        temp.path().join("b.json"),
        temp.path().join("ffm.yml"),
    );
    std::fs::write(&json_a, r#"{"mcpServers":{"svc-a":{"type":"http","url":"https://mcp.example.test/a/","headers":{"x-key":"${K}"}},"stdio-x":{"command":"ffm-mcp"},"off":{"type":"http","url":"https://mcp.example.test/off","disabled":true}}}"#).expect("a");
    std::fs::write(&json_b, r#"{"mcpServers":{"svc-a":{"type":"streamable-http","url":"https://mcp.example.test/a2","disabledTools":["drop_me"]}}}"#).expect("b");
    std::fs::write(&yml, "mcp_tls_verify: false\nmcp_servers:\n  svc-yml:\n    type: http\n    url: https://mcp.example.test/y\n").expect("yml");
    let empty = load_yaml_settings(&temp.path().join("missing.yml")).expect("empty");
    assert_eq!(empty.servers.len(), 0);
    assert!(parse_mcp_json("{bad", "bad.json", true, None).is_err());
    let extra_str = std::env::join_paths([&json_b])
        .expect("join")
        .to_string_lossy()
        .into_owned();
    assert_eq!(split_config_paths(None).len(), 0);
    let found = config::discover(&[json_a], Some(&yml), Some(&extra_str), None);
    assert_eq!(found.len(), 2);
    assert_eq!(found[0].url, "https://mcp.example.test/a2");
    assert_eq!(found[0].disabled_tools, vec!["drop_me"]);
    assert!(!found[1].tls_verify);
    drop(YamlMcpSettings::default());
}

#[test]
fn protocol_codec_handles_json_sse_errors_and_schema_defaults() {
    drop(initialize_body(1));
    drop(initialized_notification());
    drop(list_tools_body(2, Some("c1")));
    assert_eq!(
        parse_rpc_payload("application/json", "").expect("empty"),
        Value::Null
    );
    let sse = "event: message\ndata: {\"jsonrpc\":\"2.0\",\"id\":2,\"result\":{\"tools\":[{\"name\":\"keep\",\"description\":\"Ok\",\"inputSchema\":{\"type\":\"object\",\"required\":[\"q\",\"n\",\"b\",\"arr\",\"obj\"],\"properties\":{\"q\":{\"type\":\"string\"},\"n\":{\"type\":\"integer\"},\"b\":{\"type\":\"boolean\"},\"arr\":{\"type\":\"array\"},\"obj\":{\"type\":\"object\"}}}},{\"name\":\"skip\",\"description\":\"No\"}]}}\n\n";
    let decoded = parse_rpc_payload("text/event-stream", sse).expect("sse");
    let (tools, next) = decode_tools(&decoded, &["skip".to_owned()]).expect("tools");
    assert!(next.is_none() && tools.len() == 1 && tools[0].name == "keep");
    let tmpl = default_args_template(&tools[0].input_schema);
    assert!(
        tmpl.contains("\"q\":\"\"") && tmpl.contains("\"b\":false") && tmpl.contains("\"n\":0")
    );

    let call = decode_call_result(
        "svc",
        "keep",
        &json!({"isError": false, "content": [{"type": "text", "text": "ok"}]}),
    );
    assert_eq!(call.text, "ok");
    let fallback = decode_call_result("svc", "keep", &json!({"raw": 1}));
    assert!(fallback.text.contains("raw"));
    assert!(parse_rpc_payload(
        "application/json",
        r#"{"error":{"code":-32000,"message":"boom"}}"#
    )
    .is_err());
}

fn ok_rpc(body: &str, sid: Option<&str>) -> Result<RpcResponse, McpRemoteError> {
    Ok(RpcResponse {
        content_type: "application/json".into(),
        session_id: sid.map(str::to_owned),
        body: body.into(),
    })
}

#[test]
fn session_paginates_tools_and_propagates_session_id() {
    let mut rpc = StubRpc::new(vec![
        ok_rpc(r#"{"result":{}}"#, Some("sid-1")),
        ok_rpc("", None),
        ok_rpc(
            r#"{"result":{"tools":[{"name":"b"}],"nextCursor":"p2"}}"#,
            None,
        ),
        ok_rpc(r#"{"result":{"tools":[{"name":"a"}]}}"#, None),
        ok_rpc(r#"{"result":{}}"#, Some("sid-2")),
        ok_rpc("", None),
        ok_rpc(
            r#"{"result":{"content":[{"type":"text","text":"done"}]}}"#,
            None,
        ),
    ]);
    let tools = list_tools_with(&mut rpc, &[]).expect("tools");
    assert_eq!(
        tools.iter().map(|t| t.name.as_str()).collect::<Vec<_>>(),
        ["a", "b"]
    );
    let called = call_tool_with(&mut rpc, "svc", "a", &json!({})).expect("call");
    assert_eq!(called.text, "done");
    assert!(rpc.seen_sessions.contains(&Some("sid-1".to_owned())));
    assert!(rpc.seen_sessions.contains(&Some("sid-2".to_owned())));
}

#[test]
fn unavailable_returns_disabled_for_mcp_requests() {
    let srv = HttpMcpServer {
        name: "svc".into(),
        url: "https://mcp.example.test".into(),
        ..HttpMcpServer::default()
    };
    let req_list = McpRequest::ListTools(srv.clone());
    assert_eq!(req_list.busy().label(), McpBusy::ListingTools.label());
    assert!(matches!(
        unavailable(&req_list),
        McpReply::Tools(_, Err(McpRemoteError::Disabled))
    ));
    let req_call = McpRequest::CallTool {
        server: srv,
        tool: "a".into(),
        arguments: json!({}),
    };
    assert_eq!(req_call.busy().label(), McpBusy::CallingTool.label());
    assert!(matches!(
        unavailable(&req_call),
        McpReply::Called(Err(McpRemoteError::Disabled))
    ));
}
