//! Purpose: Client transport error paths, CA PEM parsing, header validation, and body limits.

use std::collections::BTreeMap;
use std::io::{self, Read, Write};
use std::net::TcpListener;
use std::thread;

use serde::{Serialize, Serializer};
use serde_json::json;

use super::super::client::response::read_rpc_body;
use super::super::client::{
    build_http, encode_body, parse_header_value, HttpRpcTransport, RpcTransport,
};
use crate::values::mcp_remote::{HttpMcpServer, McpRemoteError};

const TEST_CA_PEM: &str = concat!(
    "-----BEGIN CERTIFICATE-----\n",
    "MIIBiTCCAS+gAwIBAgIBATAKBggqhkjOPQQDAjAUMRIwEAYDVQQDDAlsb2NhbGhv\n",
    "c3QwIBcNMjQwMTAxMDAwMDAwWhgPMjEyN",
    "DAxMDEwMDAwMDBaMBQxEjAQBgNVBAMM\n",
    "CWxvY2FsaG9zdDBZMBMGByqGSM49AgEGCCqGSM49AwEHA0IABIOxeaascxR+D0/3\n",
    "GXoeeunN3XgfDz4GT9xxzA11QAB1meVkIy681sbQG0YAmlNkIsDfkEBVty3O0f+b\n",
    "FRFRk2yjcDBuMAwGA1UdEwEB/wQCMAAwDgYDVR0PAQH/BAQDAgWgMBMGA1UdJQQM\n",
    "MAoGCCsGAQUFBwMBMBoGA1UdEQQTMBGCCWxvY2FsaG9zdIcEfwAAATAdBgNVHQ4E\n",
    "FgQUO9d3n0d5OigpHM9HwoDjRYxTyhcwCgYIKoZIzj0EAwIDSAAwRQIhAOlAJiBb\n",
    "+UXk71Eb8bsgEhhsBO2hTTpK4BYjd9YW/5rdAiAuDzxBPeJH7EYbmlJz1hYIjrdO\n",
    "db0UTy3lYkek2PcU2w==\n",
    "-----END CERTIFICATE-----\n",
);

struct FailingJson;

impl Serialize for FailingJson {
    fn serialize<S: Serializer>(&self, _: S) -> Result<S::Ok, S::Error> {
        Err(serde::ser::Error::custom("boom"))
    }
}

struct ReadErr;

impl Read for ReadErr {
    fn read(&mut self, _: &mut [u8]) -> io::Result<usize> {
        Err(io::Error::other("stream aborted"))
    }
}

fn spawn_writer(
    write_body: impl FnOnce(&mut std::net::TcpStream) + Send + 'static,
) -> (String, thread::JoinHandle<()>) {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind");
    let addr = listener.local_addr().expect("addr");
    let handle = thread::spawn(move || {
        if let Ok((mut stream, _)) = listener.accept() {
            let mut buf = [0u8; 2048];
            drop(stream.read(&mut buf));
            write_body(&mut stream);
        }
    });
    (format!("http://{addr}"), handle)
}

fn test_tr(url: String) -> HttpRpcTransport {
    let srv = HttpMcpServer {
        url,
        ..Default::default()
    };
    HttpRpcTransport::with_test_url(&srv, |_| None).expect("tr")
}

#[test]
fn ca_bundle_rejects_invalid_and_empty_pem() {
    let temp = tempfile::TempDir::new().expect("temp");
    let pem = temp.path().join("ca.pem");
    let srv = HttpMcpServer {
        url: "https://mcp.example.test/svc".into(),
        ca_cert: Some(pem.clone()),
        ..Default::default()
    };
    std::fs::write(
        &pem,
        "-----BEGIN CERTIFICATE-----\n!!!\n-----END CERTIFICATE-----\n",
    )
    .expect("bad b64");
    assert!(matches!(
        HttpRpcTransport::from_server_with(&srv, |_| None),
        Err(McpRemoteError::Config(_))
    ));
    std::fs::write(&pem, "not a pem").expect("empty");
    assert!(matches!(
        HttpRpcTransport::from_server_with(&srv, |_| None),
        Err(McpRemoteError::Config(_))
    ));
}

#[test]
fn ca_bundle_rejects_bad_der_and_accepts_valid_pem() {
    assert!(matches!(
        build_http(reqwest::blocking::ClientBuilder::new().user_agent("\n")),
        Err(McpRemoteError::Network(_))
    ));
    let temp = tempfile::TempDir::new().expect("temp");
    let pem = temp.path().join("ca.pem");
    let mut srv = HttpMcpServer {
        url: "https://mcp.example.test/svc".into(),
        tls_verify: false,
        ca_cert: Some(pem.clone()),
        ..Default::default()
    };
    std::fs::write(&pem, TEST_CA_PEM).expect("valid");
    assert!(HttpRpcTransport::from_server_with(&srv, |_| None).is_ok());
    srv.tls_verify = true;
    assert!(HttpRpcTransport::from_server_with(&srv, |_| None).is_ok());
}

#[test]
fn resolved_headers_rejects_bad_name_and_value() {
    let bad_name = BTreeMap::from([("bad header".into(), "ok".into())]);
    let srv_name = HttpMcpServer {
        url: "http://127.0.0.1:1".into(),
        headers: bad_name,
        ..Default::default()
    };
    assert!(matches!(
        HttpRpcTransport::with_test_url(&srv_name, |_| None),
        Err(McpRemoteError::Config(_))
    ));

    let bad_val = BTreeMap::from([("x-test".into(), "bad\nval".into())]);
    let srv_val = HttpMcpServer {
        url: "http://127.0.0.1:1".into(),
        headers: bad_val,
        ..Default::default()
    };
    assert!(matches!(
        HttpRpcTransport::with_test_url(&srv_val, |_| None),
        Err(McpRemoteError::Config(_))
    ));
    assert!(matches!(
        parse_header_value("x-test", "bad\nval"),
        Err(McpRemoteError::Config(_))
    ));
}

#[test]
fn post_rejects_bad_session_id_and_encode_errors() {
    let mut tr = test_tr("http://127.0.0.1:1".into());
    assert!(matches!(
        tr.post(Some("bad\nid"), &json!({})),
        Err(McpRemoteError::Protocol(_))
    ));
    assert!(matches!(
        encode_body(&FailingJson),
        Err(McpRemoteError::Protocol(_))
    ));
}

#[test]
fn post_maps_send_error_and_truncated_body() {
    let (closed_url, h1) = spawn_writer(|_| {});
    let mut tr1 = test_tr(closed_url);
    assert!(matches!(
        tr1.post(None, &json!({})),
        Err(McpRemoteError::Network(_))
    ));
    drop(h1.join());

    let (trunc_url, h2) = spawn_writer(|s| {
        let head = b"HTTP/1.1 200 OK\r\nContent-Length: 100\r\nConnection: close\r\n\r\n{\"a\":";
        drop(s.write_all(head));
    });
    let mut tr2 = test_tr(trunc_url);
    assert!(matches!(
        tr2.post(None, &json!({})),
        Err(McpRemoteError::Network(_))
    ));
    drop(h2.join());
}

#[test]
fn post_rejects_oversized_response_body() {
    let (big_url, handle) = spawn_writer(|s| {
        let len = 16 * 1024 * 1024 + 1;
        let head = format!("HTTP/1.1 200 OK\r\nContent-Length: {len}\r\nConnection: close\r\n\r\n");
        drop(s.write_all(head.as_bytes()));
        drop(io::copy(&mut io::repeat(b'a').take(len), s));
    });
    let mut tr = test_tr(big_url);
    assert!(matches!(
        tr.post(None, &json!({})),
        Err(McpRemoteError::Protocol(_))
    ));
    drop(handle.join());

    assert!(matches!(
        read_rpc_body(200, String::new(), None, ReadErr, 64),
        Err(McpRemoteError::Network(_))
    ));
}
