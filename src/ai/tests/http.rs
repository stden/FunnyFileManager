//! Purpose: Loopback HTTPS/HTTP transport and CA config error tests for `ai`.

use std::collections::BTreeMap;
use std::io::{Read, Write};
use std::net::TcpListener;
use std::thread;

use serde_json::json;

use super::super::config::{ApiSecret, AuthHeader};
use super::super::*;
use crate::values::ai::{AiProvider, AiSettings, ProviderSettings};

fn spawn_one(status: &str, body: &str) -> (String, thread::JoinHandle<()>) {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind");
    let addr = listener.local_addr().expect("addr");
    let reply = format!(
        "HTTP/1.1 {status}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    );
    let handle = thread::spawn(move || {
        if let Ok((mut stream, _)) = listener.accept() {
            let mut buf = [0u8; 4096];
            drop(stream.read(&mut buf));
            drop(stream.write_all(reply.as_bytes()));
        }
    });
    (format!("http://{addr}"), handle)
}

fn sample_resolved(base_url: String) -> ResolvedProvider {
    let mut headers = BTreeMap::new();
    headers.insert("x-gateway-key".into(), "${GW_KEY}".into());
    ResolvedProvider {
        provider: AiProvider::Anthropic,
        base_url,
        model: "claude-sonnet-4-5".into(),
        max_tokens: 512,
        auth: AuthHeader::AnthropicKey,
        secret: ApiSecret::try_new_for_test("tok"),
        headers,
        tls_verify: true,
        ca_cert: None,
        system_prompt: None,
    }
}

#[test]
fn http_ai_transport_posts_and_maps_http_errors() {
    let (url, h1) = spawn_one(
        "200 OK",
        r#"{"model":"claude-sonnet-4-5","content":[{"type":"text","text":"Hi"}]}"#,
    );
    let resolved = sample_resolved(url);
    let mut tr =
        HttpAiTransport::with_test_base(&resolved, |k| (k == "GW_KEY").then(|| "gw".into()))
            .expect("transport");
    let val = tr.post("/v1/messages", &json!({})).expect("post");
    assert_eq!(
        val.get("model").and_then(|v| v.as_str()),
        Some("claude-sonnet-4-5")
    );
    drop(h1.join());

    let (err_url, h2) = spawn_one("429 Too Many Requests", "rate limit");
    let err_res = ResolvedProvider {
        base_url: err_url,
        headers: BTreeMap::new(),
        ..resolved
    };
    let mut tr2 = HttpAiTransport::with_test_base(&err_res, |_| None).expect("tr2");
    let err = tr2.post("/v1/messages", &json!({})).unwrap_err();
    assert!(matches!(err, AiError::Http(429, _)) && err.to_string().contains("429"));
    drop(h2.join());
}

#[test]
fn execute_rejects_missing_ca_bundle() {
    let mut bad_settings = AiSettings {
        ca_cert: Some("/nonexistent/ca.pem".into()),
        anthropic: ProviderSettings {
            base_url: Some("https://ai.example.test".into()),
            ..ProviderSettings::default()
        },
        ..AiSettings::default()
    };
    bad_settings.tls_verify = Some(true);
    let req = AiRequest {
        provider: AiProvider::Anthropic,
        messages: Vec::new(),
        context: WorkspaceContext::default(),
        settings: bad_settings,
    };
    assert!(execute(&req).is_err());
}

#[test]
fn http_ai_transport_builds_from_resolved_and_rejects_bad_ca() {
    let mut resolved = sample_resolved("https://api.example.test".into());
    resolved.headers.clear();
    assert!(HttpAiTransport::from_resolved(&resolved).is_ok());
    assert!(HttpAiTransport::from_resolved_with(&resolved, |_| None).is_ok());
    resolved.ca_cert = Some("/nonexistent/ca.pem".into());
    assert!(HttpAiTransport::from_resolved_with(&resolved, |_| None).is_err());
}
