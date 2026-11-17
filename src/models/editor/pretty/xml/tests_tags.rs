//! Purpose: A tag the tokenizer cannot close is refused as invalid XML rather
//! than printed as if it were complete.

use super::format_xml;

#[test]
fn an_unterminated_tag_is_refused_as_invalid_xml() {
    let error = format_xml("<root><item attr=\"x\"").unwrap_err();
    assert!(error.starts_with("Invalid XML"), "{error}");
}
