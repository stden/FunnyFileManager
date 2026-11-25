//! Purpose: Loopback HTTP transport and URL validation tests for `mcp_remote`.

use std::collections::BTreeMap;
use std::io::{Read, Write};
use std::net::TcpListener;
use std::thread;

use serde_json::json;

use super::super::client::validate_https_url;
use super::super::*;
use crate::values::mcp_remote::HttpMcpServer;

fn spawn_sequence(
    replies: Vec<(&'static str, &'static str, Option<&'static str>)>,
) -> (String, thread::JoinHandle<()>) {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind");
    let addr = listener.local_addr().expect("addr");
    let handle = thread::spawn(move || {
        for (status, body, sid) in replies {
            let Ok((mut stream, _)) = listener.accept() else {
                break;
            };
            let mut buf = [0u8; 2048];
            drop(stream.read(&mut buf));
            let sid_hdr = sid.map_or_default(|s| format!("Mcp-Session-Id: {s}\r\n"));
            let resp = format!(
                "HTTP/1.1 {status}\r\nContent-Type: application/json\r\n{sid_hdr}Content-Length: {}\r\nConnection: close\r\n\r\n{body}",
                body.len()
            );
            drop(stream.write_all(resp.as_bytes()));
        }
    });
    (format!("http://{addr}"), handle)
}

#[test]
fn http_rpc_transport_round_trips_loopback() {
    let (url, h) = spawn_sequence(vec![
        ("200 OK", r#"{"result":{}}"#, Some("s-42")),
        ("204 No Content", "", None),
        (
            "200 OK",
            r#"{"result":{"tools":[{"name":"ping","description":"Ping"}]}}"#,
            None,
        ),
    ]);
    let mut headers = BTreeMap::new();
    headers.insert("x-api-key".into(), "${API_KEY}".into());
    let server = HttpMcpServer {
        name: "svc".into(),
        url,
        headers,
        tls_verify: true,
        ..HttpMcpServer::default()
    };
    let mut tr =
        HttpRpcTransport::with_test_url(&server, |k| (k == "API_KEY").then(|| "val".into()))
            .expect("tr");
    let McpReply::Tools(_, Ok(tools)) = execute_with(&mut tr, McpRequest::ListTools(server)) else {
        panic!("expected tools");
    };
    assert_eq!(tools.len(), 1);
    assert_eq!(tools[0].name, "ping");
    drop(h.join());
}

#[test]
fn http_rpc_transport_reports_http_errors() {
    let (err_url, h2) = spawn_sequence(vec![("403 Forbidden", "denied", None)]);
    let err_srv = HttpMcpServer {
        url: err_url,
        ..HttpMcpServer::default()
    };
    let mut tr2 = HttpRpcTransport::with_test_url(&err_srv, |_| None).expect("tr2");
    let err = tr2.post(None, &json!({})).unwrap_err();
    assert!(matches!(err, McpRemoteError::Http(403, _)) && err.to_string().contains("403"));
    drop(h2.join());
}

#[test]
fn validate_https_and_execute_reject_bad_configs_and_missing_ca() {
    assert!(validate_https_url("https://mcp.example.test/svc").is_ok());
    assert!(validate_https_url("http://mcp.example.test/svc").is_err());
    assert!(validate_https_url("https://user:pass@mcp.example.test").is_err());
    assert!(validate_https_url("not a url").is_err());

    let bad = HttpMcpServer {
        name: "svc".into(),
        url: "https://mcp.example.test/svc".into(),
        ca_cert: Some("/nonexistent/ca.pem".into()),
        ..HttpMcpServer::default()
    };
    let reply = execute(McpRequest::ListTools(bad));
    assert!(matches!(
        reply,
        McpReply::Tools(_, Err(McpRemoteError::Config(_)))
    ));
    let reply_call = execute(McpRequest::CallTool {
        server: HttpMcpServer {
            name: "svc".into(),
            url: "http://bad".into(),
            ..HttpMcpServer::default()
        },
        tool: "t".into(),
        arguments: json!({}),
    });
    assert!(matches!(
        reply_call,
        McpReply::Called(Err(McpRemoteError::Config(_)))
    ));
}

#[test]
fn http_rpc_transport_builds_from_server_and_formats_call_body() {
    let ok_srv = HttpMcpServer {
        name: "svc".into(),
        url: "https://mcp.example.test/svc".into(),
        tls_verify: true,
        ..HttpMcpServer::default()
    };
    assert!(HttpRpcTransport::from_server(&ok_srv).is_ok());
    assert!(HttpRpcTransport::from_server_with(&ok_srv, |_| None).is_ok());
    let body = protocol::call_tool_body(3, "ping", &json!({"x": 1}));
    assert_eq!(body["method"], "tools/call");
}

#[test]
fn execute_with_calls_tool_over_loopback() {
    let (url, handle) = spawn_sequence(vec![
        ("200 OK", r#"{"result":{}}"#, Some("s-call")),
        ("204 No Content", "", None),
        (
            "200 OK",
            r#"{"result":{"content":[{"type":"text","text":"pong"}]}}"#,
            None,
        ),
    ]);
    let server = HttpMcpServer {
        name: "svc".into(),
        url,
        ..HttpMcpServer::default()
    };
    let mut tr = HttpRpcTransport::with_test_url(&server, |_| None).expect("tr");
    let reply = execute_with(
        &mut tr,
        McpRequest::CallTool {
            server,
            tool: "ping".into(),
            arguments: json!({}),
        },
    );
    assert!(matches!(reply, McpReply::Called(Ok(res)) if res.text == "pong"));
    drop(handle.join());
}

#[test]
fn execute_builds_transport_and_reaches_local_https_socket() {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind");
    let addr = listener.local_addr().expect("addr");
    let handle = thread::spawn(move || {
        if let Ok((stream, _)) = listener.accept() {
            drop(stream);
        }
    });
    let _no_proxy = crate::test_support::EnvVarsGuard::set(&[
        ("NO_PROXY", std::path::Path::new("127.0.0.1")),
        ("no_proxy", std::path::Path::new("127.0.0.1")),
    ]);
    let server = HttpMcpServer {
        name: "svc".into(),
        url: format!("https://{addr}/mcp"),
        tls_verify: false,
        ..HttpMcpServer::default()
    };
    let reply = execute(McpRequest::ListTools(server));
    assert!(matches!(
        reply,
        McpReply::Tools(_, Err(McpRemoteError::Network(_)))
    ));
    drop(handle.join());
}
