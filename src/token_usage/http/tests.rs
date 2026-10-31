//! Purpose: HTTP headers redact API keys and request periods remain stable across pages.
use super::*;
#[test]
fn credentials_are_sensitive_and_anthropic_uses_analytics_headers() {
    let key = ApiKey("fixture-secret".to_owned().into());
    for provider in Provider::ALL {
        let values = headers(provider, &key).expect("headers");
        let name = if provider == Provider::Anthropic {
            "x-api-key"
        } else {
            "authorization"
        };
        assert!(values.get(name).expect("credential").is_sensitive());
    }
    let values = headers(Provider::Anthropic, &key).expect("headers");
    assert_eq!(
        values.get("anthropic-version").expect("version"),
        "2023-06-01"
    );
    assert!(values.contains_key("anthropic-beta"));
}
#[test]
fn request_pages_keep_the_same_reporting_window_and_escape_cursors() {
    let end = chrono::DateTime::parse_from_rfc3339("2026-10-31T00:00:00Z")
        .expect("date")
        .to_utc();
    for provider in Provider::ALL {
        let (url, first) = request(provider, None, end);
        let (next_url, mut second) = request(provider, Some("page+/&"), end);
        assert_eq!(url, next_url);
        assert_eq!(second.pop(), Some(("page", "page+/&".to_owned())));
        assert_eq!(first, second);
        assert!(url.starts_with("https://"));
    }
}
