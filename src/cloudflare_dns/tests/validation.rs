//! Purpose: Record names, addresses, TTLs and the proxy rule are checked before any request.

use super::*;
use crate::models::cloudflare_dns::{Proxy, Ttl};

fn label() -> String {
    "a".repeat(63)
}

#[test]
fn a_record_name_is_dot_separated_letters_digits_and_hyphens() {
    for name in ["host", "a-b.c1", "www.example.com", label().as_str()] {
        assert!(record_name(name).is_ok(), "{name} is a valid name");
    }
    let refused = [
        String::new(),
        "a..b".to_owned(),
        "-a".to_owned(),
        "a-".to_owned(),
        "under_score".to_owned(),
        "a b".to_owned(),
        "host.".to_owned(),
        "a".repeat(64),
    ];
    for name in refused {
        assert!(record_name(&name).is_err(), "{name} is refused");
    }
}

#[test]
fn a_record_name_is_at_most_253_characters() {
    let too_long = [label(), label(), label(), "d".repeat(63)].join(".");
    assert!(record_name(&too_long).is_err());
    let longest = [label(), label(), label(), "d".repeat(61)].join(".");
    assert_eq!(longest.len(), 253);
    assert!(record_name(&longest).is_ok());
}

#[test]
fn an_a_record_content_is_an_ipv4_address() {
    assert_eq!(
        content("A", "203.0.113.5").ok(),
        Some("203.0.113.5".to_owned())
    );
    for text in ["256.1.1.1", "example.com", "", "2001:db8::1"] {
        assert!(content("A", text).is_err(), "{text:?} is refused");
    }
}

#[test]
fn an_aaaa_content_is_ipv6_and_other_types_need_some_text() {
    assert!(content("AAAA", "2001:db8::1").is_ok());
    assert!(content("AAAA", "203.0.113.5").is_err());
    assert!(content("TXT", "v=spf1 -all").is_ok());
    assert!(content("TXT", " ").is_err());
}

#[test]
fn a_ttl_is_automatic_at_one_or_sixty_to_eighty_six_thousand_four_hundred_seconds() {
    assert_eq!(ttl("1").ok(), Some(Ttl::Automatic));
    assert_eq!(ttl(" 60 ").ok(), Some(Ttl::Seconds(60)));
    assert_eq!(ttl("86400").ok(), Some(Ttl::Seconds(86_400)));
    for text in ["0", "59", "86401", "-1", "abc", ""] {
        assert!(ttl(text).is_err(), "{text:?} is refused");
    }
}

#[test]
fn only_a_aaaa_and_cname_records_can_be_proxied() {
    for kind in ["A", "AAAA", "CNAME"] {
        assert!(proxy(kind, Proxy::On).is_ok(), "{kind} is proxied");
    }
    for kind in ["TXT", "MX"] {
        assert!(proxy(kind, Proxy::On).is_err(), "{kind} is not");
    }
    assert!(proxy("TXT", Proxy::Off).is_ok());
    assert!(proxy("TXT", Proxy::Unsupported).is_ok());
}
