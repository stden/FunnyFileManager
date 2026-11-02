//! The settings: both variables required, the address `https://` with no user
//! information, query or fragment and trailing slashes trimmed, and the token
//! kept out of prints.

use crate::things::{Config, ConfigError, TOKEN_VARIABLE, URL_VARIABLE};

const ADDRESS: &str = "https://things.example.com";
const TOKEN: &str = "test-token-123";

fn settings(url: Option<&str>, token: Option<&str>) -> Result<Config, ConfigError> {
    Config::from_values(url.map(str::to_owned), token.map(str::to_owned))
}

fn refusal(url: Option<&str>, token: Option<&str>) -> ConfigError {
    settings(url, token).expect_err("the settings are refused")
}

#[test]
fn a_missing_address_names_its_variable() {
    assert_eq!(
        refusal(None, Some(TOKEN)),
        ConfigError::Missing(vec![URL_VARIABLE])
    );
}

#[test]
fn a_missing_token_names_its_variable() {
    assert_eq!(
        refusal(Some(ADDRESS), None),
        ConfigError::Missing(vec![TOKEN_VARIABLE])
    );
}

#[test]
fn blank_values_count_as_missing_and_both_are_named() {
    let error = refusal(Some("  "), Some(""));
    assert_eq!(
        error,
        ConfigError::Missing(vec![URL_VARIABLE, TOKEN_VARIABLE])
    );
    assert_eq!(
        error.to_string(),
        "Set THINGS_URL and THINGS_API_TOKEN to browse Things"
    );
}

#[test]
fn plain_http_is_refused_so_the_token_never_travels_in_clear_text() {
    assert_eq!(
        refusal(Some("http://things.example.com"), Some(TOKEN)),
        ConfigError::NotHttps
    );
}

#[test]
fn user_information_in_the_address_is_refused() {
    assert_eq!(
        refusal(Some("https://ops@things.example.com"), Some(TOKEN)),
        ConfigError::UserInfo
    );
    let error = refusal(Some("https://ops:hunter2@things.example.com"), Some(TOKEN));
    assert_eq!(error, ConfigError::UserInfo);
    assert!(!error.to_string().contains("hunter2"));
}

#[test]
fn an_address_that_does_not_parse_is_refused() {
    assert_eq!(refusal(Some("https://"), Some(TOKEN)), ConfigError::Address);
}

#[test]
fn a_query_or_a_fragment_in_the_address_is_refused_without_echoing_it() {
    for address in [
        "https://things.example.com?x=1",
        "https://things.example.com#frag",
    ] {
        let error = refusal(Some(address), Some(TOKEN));
        assert_eq!(error, ConfigError::QueryOrFragment);
        assert!(error.to_string().contains(URL_VARIABLE), "{error}");
        assert!(!error.to_string().contains(address), "{error}");
    }
}

#[test]
fn trailing_slashes_are_trimmed_from_the_address() {
    let config = settings(Some("https://things.example.com/// "), Some(TOKEN))
        .expect("settings with a trailing slash");
    assert_eq!(
        config.machines_url(),
        "https://things.example.com/api/v1/machines"
    );
}

#[test]
fn the_token_is_trimmed_and_a_line_break_in_it_is_refused() {
    let config = settings(Some(ADDRESS), Some("  test-token-123 \n"));
    assert_eq!(
        config
            .expect("a surrounding blank is trimmed")
            .token()
            .as_str(),
        TOKEN
    );
    assert_eq!(
        refusal(Some(ADDRESS), Some("test\ntoken")),
        ConfigError::Token
    );
}

#[test]
fn debug_prints_neither_the_address_nor_the_token() {
    let config = settings(Some(ADDRESS), Some(TOKEN)).expect("settings");
    let printed = format!("{config:?} {:?}", config.token());
    assert!(!printed.contains(TOKEN), "{printed}");
    assert!(!printed.contains("things.example.com"), "{printed}");
    assert!(printed.contains("<redacted>"), "{printed}");
}

#[test]
fn refusal_messages_name_the_variable_and_never_the_value() {
    let shown = ConfigError::NotHttps.to_string();
    assert!(shown.contains(URL_VARIABLE), "{shown}");
    let shown = ConfigError::Token.to_string();
    assert!(shown.contains(TOKEN_VARIABLE), "{shown}");
}
