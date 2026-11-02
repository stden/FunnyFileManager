//! Purpose: What `logged` may write about a failure: never text a server or a file supplied.

use super::super::{features::Diagnostic, PayloadFree};
use serde_json::json;

/// Text a language server can echo back from the user's document.
const DOCUMENT_TEXT: &str = "send the payroll export before Friday";

/// A server that sends document text where a severity number belongs: serde's
/// own message quotes that text, so the logged description must leave it out.
#[test]
fn a_rejected_json_value_is_described_without_its_text() {
    let error = serde_json::from_value::<Vec<Diagnostic>>(json!([{
        "range": {"start": {"line": 0, "character": 0}, "end": {"line": 0, "character": 3}},
        "message": "unused variable",
        "severity": DOCUMENT_TEXT,
    }]))
    .unwrap_err();

    assert!(
        error.to_string().contains(DOCUMENT_TEXT),
        "the premise: serde quotes the rejected value: {error}"
    );
    assert_eq!(error.describe(), "JSON data error");
}

#[test]
fn malformed_json_is_described_by_its_kind_and_position() {
    let error = serde_json::from_slice::<serde_json::Value>(b"{\"a\": x}").unwrap_err();

    assert_eq!(error.describe(), "JSON syntax error at line 1, column 7");
}

#[test]
fn an_error_with_a_fixed_message_is_described_by_that_message() {
    let error = "Content-Length".parse::<usize>().unwrap_err();

    assert_eq!(error.describe(), "invalid digit found in string");
}
