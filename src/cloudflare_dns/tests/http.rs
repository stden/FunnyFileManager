//! Purpose: The HTTP client sends the bearer token to the base it was given and nowhere else: it
//! follows no redirect, retries no rate limit, and shows no error body. Loopback only.

use super::*;
use crate::cloudflare_dns::client::Client;
use std::io::{BufRead, BufReader, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::mpsc;
use std::time::Duration;

const BASE_PATH: &str = "/client/v4";
const DUMMY_TOKEN: &str = "test-token-000";

fn secret() -> token::Token {
    token::Token::from_value(Some(DUMMY_TOKEN.to_owned())).expect("a token")
}

fn json_response(status: &str, body: &str) -> String {
    format!(
        "HTTP/1.1 {status}\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\
         Connection: close\r\n\r\n{body}",
        body.len()
    )
}

/// Reads the request head: the request line and the headers, one per line.
fn read_head(stream: &mut TcpStream) -> String {
    let mut lines = Vec::new();
    for line in BufReader::new(stream).lines() {
        let line = line.expect("request line");
        if line.is_empty() {
            break;
        }
        lines.push(line);
    }
    lines.join("\n")
}

/// Answers exactly one connection with `response`, then closes the listener. The head
/// it read comes back on the channel, so a test can check the request it received.
fn serve_once(response: String) -> (String, mpsc::Receiver<String>) {
    let listener = TcpListener::bind("127.0.0.1:0").expect("listener");
    let address = listener.local_addr().expect("address");
    let (sender, receiver) = mpsc::channel();
    std::thread::spawn(move || {
        let (mut stream, _) = listener.accept().expect("connection");
        drop(sender.send(read_head(&mut stream)));
        drop(stream.write_all(response.as_bytes()));
    });
    (format!("http://{address}{BASE_PATH}"), receiver)
}

#[test]
fn a_get_carries_the_bearer_token_and_the_path_under_the_base() {
    let body = r#"{"success":true,"result":[],"result_info":{"page":1,"total_pages":1}}"#;
    let (base, head) = serve_once(json_response("200 OK", body));
    let mut client = Client::with_base(secret(), &base).expect("client");
    assert_eq!(list::zones(&mut client), Ok(Vec::new()));
    let head = head
        .recv_timeout(Duration::from_secs(5))
        .expect("the request head");
    let request_line = head.lines().next().unwrap_or_default();
    assert!(
        request_line.starts_with("GET /client/v4/zones?page=1&per_page=50 "),
        "the zone list is read with a GET under the base: {request_line}"
    );
    let authorised = head
        .lines()
        .any(|line| line.eq_ignore_ascii_case("authorization: Bearer test-token-000"));
    assert!(authorised, "the bearer header is sent");
}

#[test]
fn a_rate_limit_is_reported_without_a_retry() {
    let (base, _head) = serve_once(json_response("429 Too Many Requests", "{}"));
    let mut client = Client::with_base(secret(), &base).expect("client");
    assert_eq!(list::zones(&mut client), Err(CloudflareError::RateLimited));
}

#[test]
fn a_redirect_to_another_host_is_not_followed() {
    let elsewhere = TcpListener::bind("127.0.0.1:0").expect("listener");
    elsewhere.set_nonblocking(true).expect("nonblocking");
    let location = format!(
        "http://{}/elsewhere",
        elsewhere.local_addr().expect("address")
    );
    let response = format!(
        "HTTP/1.1 302 Found\r\nLocation: {location}\r\nContent-Length: 0\r\n\
         Connection: close\r\n\r\n"
    );
    let (base, _head) = serve_once(response);
    let mut client = Client::with_base(secret(), &base).expect("client");
    assert_eq!(list::zones(&mut client), Err(CloudflareError::Status(302)));
    assert!(
        elsewhere.accept().is_err(),
        "nothing connected to the redirect target"
    );
}

#[test]
fn an_error_envelope_is_reported_by_its_first_error_only() {
    let body = r#"{"success":false,"errors":[{"code":10000,"message":"Authentication error"}],"result":null}"#;
    let (base, _head) = serve_once(json_response("403 Forbidden", body));
    let mut client = Client::with_base(secret(), &base).expect("client");
    let error = list::zones(&mut client).expect_err("a refusal");
    assert_eq!(
        error,
        CloudflareError::Api {
            code: 10000,
            message: "Authentication error".to_owned(),
        }
    );
    assert!(
        !error.message().contains(DUMMY_TOKEN),
        "no token in the message"
    );
}

#[test]
fn a_refusal_without_an_envelope_is_an_access_denial() {
    let (base, _head) = serve_once(json_response("401 Unauthorized", "not json"));
    let mut client = Client::with_base(secret(), &base).expect("client");
    assert_eq!(list::zones(&mut client), Err(CloudflareError::AccessDenied));
}
