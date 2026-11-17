//! Purpose: The token is refused when blank, sent only as a bearer value, and never printed.

use super::*;

const TOKEN: &str = "test-token-000";

#[test]
fn a_missing_or_blank_variable_is_the_missing_token_error() {
    for value in [None, Some(String::new()), Some("   ".to_owned())] {
        let error = token::Token::from_value(value).err();
        assert_eq!(error, Some(CloudflareError::MissingToken));
    }
    assert_eq!(
        CloudflareError::MissingToken.message(),
        "set CLOUDFLARE_API_TOKEN"
    );
}

#[test]
fn the_token_is_hidden_in_debug_and_display_and_sent_as_a_bearer_value() {
    let token = token::Token::from_value(Some(format!(" {TOKEN} "))).expect("a token");
    assert!(
        !format!("{token:?}").contains(TOKEN),
        "debug output hides the token"
    );
    assert!(
        !format!("{token}").contains(TOKEN),
        "display output hides the token"
    );
    assert!(
        token.authorization().as_str() == format!("Bearer {TOKEN}"),
        "the authorization value is the trimmed token, as a bearer value"
    );
}
