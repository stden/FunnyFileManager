//! Purpose: The token and the client are read from the environment, and a blank variable is refused.

use super::client::Client;
use super::token::Token;
use super::*;
use crate::values::cloudflare_dns::TOKEN_VARIABLE;

#[test]
fn the_token_and_the_client_come_from_the_environment_and_a_blank_variable_is_refused() {
    let _lock = crate::test_support::env_lock();
    let previous = std::env::var(TOKEN_VARIABLE).ok();
    std::env::set_var(TOKEN_VARIABLE, "  token-value  ");
    let token = Token::from_environment();
    let client = Client::from_environment();
    std::env::set_var(TOKEN_VARIABLE, "   ");
    let blank = Token::from_environment();
    match previous {
        Some(value) => std::env::set_var(TOKEN_VARIABLE, value),
        None => std::env::remove_var(TOKEN_VARIABLE),
    }
    assert!(token.is_ok(), "a set variable gives a token");
    assert!(client.is_ok(), "a set variable gives a client");
    assert!(
        matches!(blank, Err(CloudflareError::MissingToken)),
        "a blank variable is refused"
    );
}

#[test]
fn a_request_without_a_token_is_answered_with_the_missing_token() {
    let _lock = crate::test_support::env_lock();
    let previous = std::env::var(TOKEN_VARIABLE).ok();
    std::env::remove_var(TOKEN_VARIABLE);
    let reply = fetch(Request::Zones);
    if let Some(value) = previous {
        std::env::set_var(TOKEN_VARIABLE, value);
    }
    assert!(matches!(
        reply,
        Reply::Zones(Err(CloudflareError::MissingToken))
    ));
}

#[test]
fn a_request_with_a_token_reads_the_zones_from_the_api() {
    let _lock = crate::test_support::env_lock();
    let previous = std::env::var(TOKEN_VARIABLE).ok();
    std::env::set_var(TOKEN_VARIABLE, "test-token-000");
    let body = serde_json::json!({
        "success": true,
        "errors": [],
        "result": [{"id": "0000000000000000000000000000000a", "name": "example.com"}],
        "result_info": {"page": 1, "total_pages": 1}
    })
    .to_string();
    let (base, _head) = http::serve_once(http::json_response("200 OK", &body));
    client::TEST_API_BASE.with(|cell| *cell.borrow_mut() = Some(base));
    let reply = fetch(Request::Zones);
    match previous {
        Some(value) => std::env::set_var(TOKEN_VARIABLE, value),
        None => std::env::remove_var(TOKEN_VARIABLE),
    }
    assert!(
        matches!(&reply, Reply::Zones(Ok(zones)) if zones.len() == 1),
        "the zones come from the API"
    );
}
