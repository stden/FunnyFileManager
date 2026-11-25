//! Purpose: Edge-case tests for AI CA bundles, header validation, and HTTP response limits.

use std::collections::BTreeMap;
use std::io::{self, Read, Write};
use std::net::{Shutdown, TcpListener};
use std::thread;

use base64::Engine as _;
use serde_json::json;

use super::super::client::{build_client, decode_body_limited};
use super::super::config::{ApiSecret, AuthHeader, ResolvedProvider};
use crate::values::ai::{AiError, AiProvider};

fn write_test_ca_pem(path: &std::path::Path) {
    let hex = concat!(
        "3082018430820129a00302010202140459d19b2b97ee37262d2b6f2199953861",
        "9e8d51300a06082a8648ce3d04030230173115301306035504030c0c6578616d",
        "706c652e74657374301e170d3236313132343132333234325a170d3336313132",
        "313132333234325a30173115301306035504030c0c6578616d706c652e746573",
        "743059301306072a8648ce3d020106082a8648ce3d030107034200044c32b3c0",
        "60bc1b494def7b1189e79943a58f7225c344ef2d9bed77383364c3f06820dc87",
        "2f6669aee02fbb44dd7463f5b088c1bf6eb2bfcf169ca05ce28fd815a3533051",
        "301d0603551d0e041604143327b58be8864388a281908d59588edaee9d5a6e30",
        "1f0603551d230418301680143327b58be8864388a281908d59588edaee9d5a6e",
        "300f0603551d130101ff040530030101ff300a06082a8648ce3d040302034900",
        "3046022100894ecc7fed06975b22c40d1bc37e4fb2446d47f3f48abeb51f7d64",
        "c568082d64022100f90d15a4b24e180a6819605f4ee3c2350c60238a82b9a93d",
        "732fc681fb4a2418",
    );
    let (pairs, _) = hex.as_bytes().as_chunks::<2>();
    let der: Vec<u8> = pairs
        .iter()
        .map(|pair| u8::from_str_radix(std::str::from_utf8(pair).expect("utf8"), 16).expect("hex"))
        .collect();
    let encoded = base64::engine::general_purpose::STANDARD.encode(der);
    let pem = format!("-----BEGIN CERTIFICATE-----\n{encoded}\n-----END CERTIFICATE-----\n");
    std::fs::write(path, pem).expect("valid pem");
}

struct BrokenStream;

impl Read for BrokenStream {
    fn read(&mut self, _: &mut [u8]) -> io::Result<usize> {
        Err(io::Error::other("broken"))
    }
}

fn spawn_raw(raw: &'static [u8]) -> (String, thread::JoinHandle<()>) {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind");
    let addr = listener.local_addr().expect("addr");
    let handle = thread::spawn(move || {
        if let Ok((mut stream, _)) = listener.accept() {
            let mut req = [0u8; 2048];
            drop(stream.read(&mut req));
            if !raw.is_empty() {
                drop(stream.write_all(raw));
                drop(stream.flush());
            }
            drop(stream.shutdown(Shutdown::Write));
        }
    });
    (format!("http://{addr}"), handle)
}

fn openai_resolved(base_url: String) -> ResolvedProvider {
    ResolvedProvider {
        provider: AiProvider::OpenAi,
        base_url,
        model: "gpt-5".into(),
        max_tokens: 512,
        auth: AuthHeader::Bearer,
        secret: ApiSecret::try_new_for_test("tok"),
        headers: BTreeMap::new(),
        tls_verify: true,
        ca_cert: None,
        system_prompt: None,
    }
}

#[test]
fn transport_rejects_bad_ca_pem_and_builder_errors() {
    let temp = tempfile::TempDir::new().expect("temp");
    let ca_path = temp.path().join("ca.pem");
    let mut resolved = openai_resolved("https://api.example.test".into());
    resolved.ca_cert = Some(ca_path.clone());

    std::fs::write(
        &ca_path,
        "-----BEGIN CERTIFICATE-----\n!!!\n-----END CERTIFICATE-----\n",
    )
    .expect("bad pem");
    assert!(
        super::super::client::HttpAiTransport::from_resolved_with(&resolved, |_| None).is_err()
    );
    std::fs::write(&ca_path, "not a certificate").expect("empty");
    assert!(
        super::super::client::HttpAiTransport::from_resolved_with(&resolved, |_| None).is_err()
    );
    assert!(build_client(reqwest::blocking::ClientBuilder::new().user_agent("\n")).is_err());
}

#[test]
fn transport_accepts_valid_ca_pem_bundle() {
    let temp = tempfile::TempDir::new().expect("temp");
    let ca_path = temp.path().join("ca.pem");
    write_test_ca_pem(&ca_path);
    let mut resolved = openai_resolved("https://api.example.test".into());
    resolved.ca_cert = Some(ca_path);
    assert!(super::super::client::HttpAiTransport::from_resolved_with(&resolved, |_| None).is_ok());
}

#[test]
fn transport_rejects_bad_headers_and_bad_secret() {
    let mut resolved = openai_resolved("https://api.example.test".into());
    resolved.secret = ApiSecret::try_new_for_test("bad\nval");
    assert!(matches!(
        super::super::client::HttpAiTransport::with_test_base(&resolved, |_| None),
        Err(AiError::Config(_))
    ));
    resolved.secret = ApiSecret::try_new_for_test("tok");

    for (name, val) in [
        ("bad header", "v"),
        ("x-ok", "${MISSING_KEY}"),
        ("x-ok", "bad\nval"),
    ] {
        resolved.headers.clear();
        resolved.headers.insert(name.into(), val.into());
        assert!(matches!(
            super::super::client::HttpAiTransport::with_test_base(&resolved, |_| None),
            Err(AiError::Config(_))
        ));
    }
    assert!(matches!(
        super::super::client::parse_header_value("x-ok", "bad\nval"),
        Err(AiError::Config(_))
    ));
}

#[test]
fn transport_supports_bearer_and_reports_closed_or_truncated_streams() {
    let (ok_url, h0) =
        spawn_raw(b"HTTP/1.1 200 OK\r\nContent-Length: 2\r\nConnection: close\r\n\r\n{}");
    let mut tr0 =
        super::super::client::HttpAiTransport::with_test_base(&openai_resolved(ok_url), |_| None)
            .expect("t0");
    assert_eq!(
        super::super::client::AiTransport::post(&mut tr0, "/v1/chat/completions", &json!({}))
            .expect("ok"),
        json!({})
    );
    drop(h0.join());

    for raw in [
        b"".as_slice(),
        b"HTTP/1.1 200 OK\r\nContent-Length: 50\r\n\r\nhi".as_slice(),
    ] {
        let (url, h) = spawn_raw(raw);
        let mut tr =
            super::super::client::HttpAiTransport::with_test_base(&openai_resolved(url), |_| None)
                .expect("tr");
        assert!(matches!(
            super::super::client::AiTransport::post(&mut tr, "/v1/chat/completions", &json!({})),
            Err(AiError::Network(_))
        ));
        drop(h.join());
    }
}

#[test]
fn transport_rejects_invalid_json_and_oversized_response_body() {
    let (bad_url, h1) =
        spawn_raw(b"HTTP/1.1 200 OK\r\nContent-Length: 8\r\nConnection: close\r\n\r\nnot-json");
    let mut tr =
        super::super::client::HttpAiTransport::with_test_base(&openai_resolved(bad_url), |_| None)
            .expect("tr");
    assert!(matches!(
        super::super::client::AiTransport::post(&mut tr, "/v1/chat/completions", &json!({})),
        Err(AiError::InvalidResponse(_))
    ));
    drop(h1.join());

    assert!(matches!(
        decode_body_limited(200, BrokenStream, 64),
        Err(AiError::Network(_))
    ));
    assert!(matches!(
        decode_body_limited(200, b"0123456789".as_slice(), 4),
        Err(AiError::InvalidResponse(_))
    ));
}
