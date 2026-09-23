//! Purpose: Standard completion, hover, definition, and diagnostic result forms.

use super::super::{
    features::*,
    positions::{Position, Range},
    types::Feature,
    uri,
};
use serde_json::json;

fn range() -> serde_json::Value {
    json!({"start":{"line":0,"character":1},"end":{"line":0,"character":3}})
}

#[test]
fn completion_arrays_and_lists_preserve_edits_details_and_documentation() {
    let item = json!({"label":"hello","kind":3,"detail":"fn hello()", "documentation":{"kind":"markdown","value":"docs"},
        "textEdit":{"range":range(),"newText":"hello"}, "additionalTextEdits":[{"range":range(),"newText":"extra"}]});
    for value in [json!([item]), json!({"isIncomplete":true,"items":[item]})] {
        let Reply::Completions(items) = parse(Feature::Completion, &value).unwrap() else {
            panic!("completion")
        };
        assert_eq!(items[0].label, "hello");
        assert_eq!(items[0].kind, Some(3));
        assert_eq!(items[0].detail, "fn hello()");
        assert_eq!(items[0].documentation, "docs");
        assert_eq!(items[0].edit.as_ref().unwrap().new_text, "hello");
        assert_eq!(items[0].additional_edits[0].new_text, "extra");
        assert!(!items[0].snippet);
    }
}

#[test]
fn insertion_fallback_snippets_and_insert_replace_edits_are_distinguished() {
    let Reply::Completions(items) = parse(Feature::Completion, &json!([
        {"label":"name"}, {"label":"function","insertText":"call($0)","insertTextFormat":2},
        {"label":"replace","textEdit":{"insert":range(),"replace":range(),"newText":"replacement"}}
    ])).unwrap() else { panic!("completion") };
    assert_eq!(items[0].insert_text, "name");
    assert!(items[1].snippet);
    assert_eq!(items[1].insert_text, "call($0)");
    assert_eq!(items[2].edit.as_ref().unwrap().new_text, "replacement");
    assert_eq!(
        parse(Feature::Completion, &json!(null)).unwrap(),
        Reply::Completions(vec![])
    );
}

#[test]
fn hover_preserves_plaintext_markdown_code_blocks_and_legacy_arrays() {
    for (contents, expected) in [
        (json!("plain"), "plain"),
        (json!({"kind":"markdown","value":"**type**"}), "**type**"),
        (
            json!({"language":"rust","value":"fn hello()"}),
            "```rust\nfn hello()\n```",
        ),
        (
            json!(["one", {"kind":"plaintext","value":"two"}]),
            "one\n\ntwo",
        ),
    ] {
        assert_eq!(
            parse(Feature::Hover, &json!({"contents":contents})).unwrap(),
            Reply::Hover(expected.into())
        );
    }
    assert_eq!(
        parse(Feature::Hover, &json!(null)).unwrap(),
        Reply::Hover(String::new())
    );
}

#[test]
fn definition_accepts_location_arrays_and_location_links() {
    let temp = tempfile::TempDir::new().unwrap();
    let path = temp.path().join("target.rs");
    let uri = uri::from_path(&path).unwrap();
    let location = json!({"uri":uri,"range":range()});
    let link = json!({"targetUri":uri,"targetRange":range(),"targetSelectionRange":range()});
    for value in [location.clone(), json!([location]), json!([link])] {
        let Reply::Definitions(targets) = parse(Feature::Definition, &value).unwrap() else {
            panic!("definitions")
        };
        assert_eq!(targets[0].path, path);
        assert_eq!(targets[0].range.start.character, 1);
        assert_eq!(targets[0].range.end.character, 3);
    }
    let Reply::Definitions(targets) = parse(Feature::Definition, &json!([link, link])).unwrap()
    else {
        panic!("definitions")
    };
    assert_eq!(targets.len(), 2);
    assert_eq!(
        parse(Feature::Definition, &json!(null)).unwrap(),
        Reply::Definitions(vec![])
    );
}

#[test]
fn malformed_edits_and_non_file_definitions_are_errors() {
    for value in [
        json!(42),
        json!([{}]),
        json!([{"label":"x","textEdit":{}}]),
        json!([{"label":"x","textEdit":{"range":{},"newText":"a"}}]),
        json!([{"label":"x","textEdit":{"range":range()}}]),
        json!([{"label":"x","additionalTextEdits":[{}]}]),
    ] {
        assert!(parse(Feature::Completion, &value).is_err());
    }
    for value in [
        json!({}),
        json!({"uri":"https://example.com/a.rs","range":range()}),
        json!({"uri":"file:///tmp/a.rs","range":{}}),
    ] {
        assert!(parse(Feature::Definition, &value).is_err());
    }
}

#[test]
fn diagnostic_labels_distinguish_errors_warnings_information_and_hints() {
    for (severity, label) in [
        (Some(1), "Error"),
        (Some(2), "Warning"),
        (Some(3), "Info"),
        (Some(4), "Hint"),
        (None, "Info"),
    ] {
        let diagnostic = Diagnostic {
            range: Range {
                start: Position::default(),
                end: Position::default(),
            },
            message: "message".into(),
            severity,
        };
        assert_eq!(diagnostic.label(), label);
    }
}
#[test]
fn markup_supports_plain_legacy_and_empty_values() {
    assert_eq!(
        super::super::features::markup(&serde_json::json!("plain")),
        "plain"
    );
    assert_eq!(super::super::features::markup(&serde_json::Value::Null), "");
}
