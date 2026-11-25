//! The request's header carries the token as a sensitive bearer credential, and
//! no failure message names it.

use crate::things::client::auth_headers;
use crate::things::{Config, ThingsError};

const TOKEN: &str = "test-token-123";

fn config() -> Config {
    Config::from_values(
        Some("https://things.example.com".to_owned()),
        Some(TOKEN.to_owned()),
    )
    .expect("settings")
}

#[test]
fn the_authorization_header_is_a_sensitive_bearer_token() {
    let headers = auth_headers(&config()).expect("headers");
    let value = headers.get("authorization").expect("the header is set");
    assert_eq!(value.to_str().expect("ASCII"), format!("Bearer {TOKEN}"));
    assert!(value.is_sensitive());
}

#[test]
fn no_failure_message_names_the_token() {
    for error in [
        ThingsError::Disabled,
        ThingsError::Request,
        ThingsError::Response,
        ThingsError::Limit,
        ThingsError::Worker,
    ] {
        assert!(!error.message().contains(TOKEN), "{error:?}");
    }
}

#[test]
fn fetch_reports_connection_refusal_and_decode_response_enforces_status_and_size() {
    use crate::things::client::decode_response;
    use reqwest::StatusCode;
    use std::io::Read;

    let local = Config::from_values(
        Some("https://127.0.0.1:1".to_owned()),
        Some(TOKEN.to_owned()),
    )
    .expect("loopback config");
    assert_eq!(crate::things::fetch(&local), Err(ThingsError::Request));

    assert_eq!(
        decode_response(StatusCode::SERVICE_UNAVAILABLE, &b"down"[..]),
        Err(ThingsError::Request)
    );
    let oversized = std::io::repeat(b' ').take(8 * 1024 * 1024 + 1);
    assert_eq!(
        decode_response(StatusCode::OK, oversized),
        Err(ThingsError::Limit)
    );
    let ok_json = br#"[{"id":"m1","name":"web","hostname":"web.example"}]"#;
    let listing = decode_response(StatusCode::OK, &ok_json[..]).expect("parsed");
    assert_eq!(listing.machines.len(), 1);
}
