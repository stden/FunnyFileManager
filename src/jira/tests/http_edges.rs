//! Purpose: Edge-case tests for Jira CA bundles, request headers, size limits, and `execute()`.

use std::io::{self, Read, Write};
use std::net::{Shutdown, TcpListener};
use std::path::Path;
use std::thread;

use base64::Engine as _;

use super::super::client::{build_client, decode_response_limited, HttpTransport, Transport};
use super::super::{config, execute, RuntimeConfig};
use crate::values::jira::{JiraError, JiraReply, JiraRequest};

fn sample_ca_der() -> Vec<u8> {
    let digits = concat!(
        "3082018c30820133a0030201020214114ab74f6b6a90d159fb58996fa14fcd9e",
        "215cf7300a06082a8648ce3d040302301c311a301806035504030c116a697261",
        "2e6578616d706c652e74657374301e170d3236313132343132333932335a170d",
        "3336313132313132333932335a301c311a301806035504030c116a6972612e65",
        "78616d706c652e746573743059301306072a8648ce3d020106082a8648ce3d03",
        "01070342000498f5710ecba8c9dc62bf234b791041e05b889c4a7fddd038116f",
        "b4de5cb758051de427a218e2067328de86243b19032c88ea904ebc2076799e58",
        "867e416c1e48a3533051301d0603551d0e0416041456fa17fda60876241f09df",
        "1025b01a21d07e01ed301f0603551d2304183016801456fa17fda60876241f09",
        "df1025b01a21d07e01ed300f0603551d130101ff040530030101ff300a06082a",
        "8648ce3d0403020347003044022055b089b04c86870c9415075630b12ad774b7",
        "483bb125b64f27e49c0fa7ead5db02201a4a723525c68443174b87759aabe9a1",
        "f747b6a0083a0b9e9e49cad609b408f3",
    );
    let (pairs, _) = digits.as_bytes().as_chunks::<2>();
    pairs
        .iter()
        .map(|pair| u8::from_str_radix(std::str::from_utf8(pair).expect("utf8"), 16).expect("hex"))
        .collect()
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

#[test]
fn http_transport_rejects_invalid_ca_pem_and_builder_errors() {
    let temp = tempfile::TempDir::new().expect("temp");
    let ca_path = temp.path().join("ca.pem");
    let cfg = RuntimeConfig {
        base_url: "https://jira.example.test".into(),
        token: config::Token::try_new_for_test("tok"),
        tls_verify: true,
        ca_cert: Some(ca_path.clone()),
        page_size: 50,
    };
    std::fs::write(
        &ca_path,
        "-----BEGIN CERTIFICATE-----\n!!!\n-----END CERTIFICATE-----\n",
    )
    .expect("bad");
    assert!(matches!(
        HttpTransport::new(&cfg),
        Err(JiraError::Config(_))
    ));
    std::fs::write(&ca_path, "not a certificate").expect("empty");
    assert!(matches!(
        HttpTransport::new(&cfg),
        Err(JiraError::Config(_))
    ));
    assert!(build_client(reqwest::blocking::ClientBuilder::new().user_agent("\n")).is_err());
}

#[test]
fn http_transport_accepts_valid_ca_pem_and_no_ca() {
    let temp = tempfile::TempDir::new().expect("temp");
    let ca_path = temp.path().join("ca.pem");
    let encoded = base64::engine::general_purpose::STANDARD.encode(sample_ca_der());
    let pem = format!("-----BEGIN CERTIFICATE-----\n{encoded}\n-----END CERTIFICATE-----\n");
    std::fs::write(&ca_path, pem).expect("valid pem");
    let mut cfg = RuntimeConfig {
        base_url: "https://jira.example.test".into(),
        token: config::Token::try_new_for_test("tok"),
        tls_verify: true,
        ca_cert: Some(ca_path),
        page_size: 50,
    };
    assert!(HttpTransport::new(&cfg).is_ok());
    cfg.ca_cert = None;
    assert!(HttpTransport::new(&cfg).is_ok());
}

#[test]
fn http_transport_rejects_bad_headers_and_closed_loopback() {
    let mut bad_tok = HttpTransport::with_base(
        config::Token::try_new_for_test("bad\ntok"),
        "http://127.0.0.1:1".into(),
    )
    .expect("client");
    assert!(matches!(bad_tok.get("/x"), Err(JiraError::Config(_))));

    let (close_url, h1) = spawn_raw(b"");
    let mut closed =
        HttpTransport::with_base(config::Token::try_new_for_test("tok"), close_url).expect("c");
    assert!(matches!(closed.get("/x"), Err(JiraError::Network(_))));
    drop(h1.join());
}

#[test]
fn http_transport_rejects_truncated_body_and_invalid_json() {
    let (trunc_url, h2) = spawn_raw(b"HTTP/1.1 200 OK\r\nContent-Length: 50\r\n\r\nhi");
    let mut trunc =
        HttpTransport::with_base(config::Token::try_new_for_test("tok"), trunc_url).expect("t");
    assert!(matches!(trunc.get("/x"), Err(JiraError::Network(_))));
    drop(h2.join());

    let (bad_json_url, h3) =
        spawn_raw(b"HTTP/1.1 200 OK\r\nContent-Length: 8\r\nConnection: close\r\n\r\nnot-json");
    let mut bad_json =
        HttpTransport::with_base(config::Token::try_new_for_test("tok"), bad_json_url).expect("j");
    assert!(matches!(
        bad_json.get("/x"),
        Err(JiraError::InvalidResponse(_))
    ));
    drop(h3.join());
}

#[test]
fn decode_response_limited_rejects_oversized_body_and_read_errors() {
    struct FaultReader;
    impl Read for FaultReader {
        fn read(&mut self, _: &mut [u8]) -> io::Result<usize> {
            Err(io::Error::other("socket reset"))
        }
    }
    assert!(matches!(
        decode_response_limited(200, FaultReader, 64),
        Err(JiraError::Network(_))
    ));
    assert!(matches!(
        decode_response_limited(200, b"0123456789".as_slice(), 4),
        Err(JiraError::InvalidResponse(_))
    ));
}

#[test]
fn execute_builds_transport_and_reaches_loopback_error() {
    let (url, h) = spawn_raw(b"");
    let _proxy = crate::test_support::EnvVarsGuard::set(&[
        ("NO_PROXY", Path::new("*")),
        ("HTTPS_PROXY", Path::new("")),
        ("https_proxy", Path::new("")),
        ("ALL_PROXY", Path::new("")),
        ("all_proxy", Path::new("")),
    ]);
    let cfg = RuntimeConfig {
        base_url: url.replacen("http://", "https://", 1),
        token: config::Token::try_new_for_test("tok"),
        tls_verify: false,
        ca_cert: None,
        page_size: 10,
    };
    let req = JiraRequest::Fetch {
        jql: "x".into(),
        page_size: 10,
    };
    assert!(matches!(
        execute(&cfg, req),
        JiraReply::Fetched(Err(JiraError::Network(_)))
    ));
    drop(h.join());
}
