//! The catalog HTTP exchange is bounded and keeps transport failures generic.

use std::time::Duration;

use reqwest::{blocking::Client, header::HeaderMap};

use super::{auth_headers, fetch, request, Config, ThingsError, MAX_BYTES};

mod server;

fn config() -> Config {
    Config::from_values(
        Some("https://things.example.com".to_owned()),
        Some("fixture-secret".to_owned()),
    )
    .expect("catalog settings")
}

fn read(reply: Vec<u8>) -> (Result<super::Listing, ThingsError>, String) {
    let (url, server) = server::serve(reply);
    let builder = Client::builder()
        .no_proxy()
        .redirect(reqwest::redirect::Policy::none())
        .timeout(Duration::from_secs(5));
    let result = request(builder, &url, auth_headers(&config()).expect("headers"));
    (result, server.join().expect("HTTP fixture"))
}

#[test]
fn machine_list_is_read_with_the_bearer_credential_and_catalog_path() {
    let (result, sent) = read(server::response(200, br#"[{"id":"7","name":"web-01"}]"#));
    assert_eq!(result.expect("machine list").machines[0].name, "web-01");
    let sent = sent.to_ascii_lowercase();
    assert!(sent.starts_with("get /api/v1/machines http/1.1\r\n"));
    assert!(sent.contains("authorization: bearer fixture-secret\r\n"));
}

#[test]
fn non_success_statuses_are_request_failures() {
    for status in [302, 401, 500] {
        assert_eq!(
            read(server::response(status, b"[]")).0.err(),
            Some(ThingsError::Request)
        );
    }
}

#[test]
fn successful_status_with_malformed_json_is_a_response_failure() {
    assert_eq!(
        read(server::response(200, b"not JSON")).0.err(),
        Some(ThingsError::Response)
    );
}

#[test]
fn truncated_response_body_is_a_request_failure() {
    let reply = b"HTTP/1.1 200 OK\r\nContent-Length: 100\r\nConnection: close\r\n\r\n[]".to_vec();
    assert_eq!(read(reply).0.err(), Some(ThingsError::Request));
}

#[test]
fn oversized_body_is_rejected_before_parsing() {
    let body = vec![b' '; usize::try_from(MAX_BYTES).expect("body limit") + 1];
    assert_eq!(
        read(server::response(200, &body)).0.err(),
        Some(ThingsError::Limit)
    );
}

#[test]
fn invalid_client_configuration_is_a_request_failure() {
    let builder = Client::builder().user_agent("invalid\r\nagent");
    assert_eq!(
        request(builder, "http://127.0.0.1:0", HeaderMap::default()).err(),
        Some(ThingsError::Request)
    );
}

#[test]
fn public_fetch_reports_local_connection_failure_without_exposing_the_token() {
    let _proxy = crate::test_support::EnvVarGuard::set("NO_PROXY", std::path::Path::new("*"));
    let config = Config::from_values(
        Some("https://127.0.0.1:0".to_owned()),
        Some("fixture-secret".to_owned()),
    )
    .expect("local HTTPS settings");
    let error = fetch(&config).expect_err("closed local endpoint");
    assert_eq!(error, ThingsError::Request);
    assert!(!error.message().contains("fixture-secret"));
}
