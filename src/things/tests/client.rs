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
