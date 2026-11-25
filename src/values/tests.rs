//! Purpose: Unit tests for shared pure value helpers (`non_blank`, `parse_tls_verify`).

use super::*;

#[test]
fn non_blank_and_parse_tls_verify_handle_whitespace_and_flags() {
    assert_eq!(non_blank(Some("  hi ".into())), Some("hi".into()));
    assert_eq!(non_blank(Some("   ".into())), None);
    assert!(!parse_tls_verify(Some("false".into()), Some(true)));
    assert!(parse_tls_verify(Some("1".into()), Some(false)));
    assert!(!parse_tls_verify(Some("unknown".into()), Some(false)));
}
