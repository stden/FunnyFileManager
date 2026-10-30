//! Unit tests for JSON and XML formatting in the editor.

use std::path::PathBuf;

use crate::models::{EditState, Editor};

#[test]
fn formats_compact_json_and_preserves_key_order() {
    let mut editor = Editor::from_content(
        "config.json".to_owned(),
        PathBuf::from("config.json"),
        r#"{"zeta":1,"alpha":{"nested":[true,false]}}"#,
    );
    assert_eq!(editor.state, EditState::Clean);

    editor.format_document();

    assert_eq!(editor.state, EditState::Modified);
    assert_eq!(editor.notice.as_deref(), Some("Formatted JSON"));
    assert_eq!(
        editor.lines,
        vec![
            "{",
            "  \"zeta\": 1,",
            "  \"alpha\": {",
            "    \"nested\": [",
            "      true,",
            "      false",
            "    ]",
            "  }",
            "}"
        ]
    );
}

#[test]
fn formats_compact_xml_with_declarations_comments_and_leaf_elements() {
    let mut editor = Editor::from_content(
        "pom.xml".to_owned(),
        PathBuf::from("pom.xml"),
        r#"<?xml version="1.0" encoding="UTF-8"?><!DOCTYPE project [<!ELEMENT project (groupId,empty)>]><project><groupId>com.example</groupId><!-- comment --><empty attr="a > b"/></project>"#,
    );

    editor.format_document();

    assert_eq!(editor.state, EditState::Modified);
    assert_eq!(editor.notice.as_deref(), Some("Formatted XML"));
    assert_eq!(
        editor.lines,
        vec![
            r#"<?xml version="1.0" encoding="UTF-8"?>"#,
            "<!DOCTYPE project [<!ELEMENT project (groupId,empty)>]>",
            "<project>",
            "  <groupId>com.example</groupId>",
            "  <!-- comment -->",
            r#"  <empty attr="a > b"/>"#,
            "</project>",
        ]
    );
}

#[test]
fn deeply_nested_xml_is_rejected_before_stack_overflow() {
    let open = "<a>".repeat(300);
    let close = "</a>".repeat(300);
    let mut editor = Editor::from_content(
        "deep.xml".to_owned(),
        PathBuf::from("deep.xml"),
        &format!("{open}{close}"),
    );
    editor.format_document();
    assert_eq!(editor.state, EditState::Clean);
    assert!(editor
        .notice
        .as_deref()
        .is_some_and(|msg| msg.contains("nesting depth exceeds")));
}

#[test]
fn invalid_json_or_xml_sets_error_notice_without_modifying_buffer() {
    let mut json_editor = Editor::from_content(
        "broken.json".to_owned(),
        PathBuf::from("broken.json"),
        r#"{"unclosed": 1"#,
    );
    json_editor.format_document();
    assert_eq!(json_editor.state, EditState::Clean);
    assert_eq!(json_editor.lines, vec![r#"{"unclosed": 1"#]);
    assert!(json_editor
        .notice
        .as_deref()
        .is_some_and(|msg| msg.starts_with("Invalid JSON:")));

    let mut xml_editor = Editor::from_content(
        "broken.xml".to_owned(),
        PathBuf::from("broken.xml"),
        "<root><unclosed></root>",
    );
    xml_editor.format_document();
    assert_eq!(xml_editor.state, EditState::Clean);
    assert_eq!(xml_editor.lines, vec!["<root><unclosed></root>"]);
    assert!(xml_editor
        .notice
        .as_deref()
        .is_some_and(|msg| msg.starts_with("Invalid XML:")));
}

#[test]
fn non_json_and_non_xml_file_reports_notice_and_leaves_buffer_untouched() {
    let mut editor = Editor::from_content(
        "notes.txt".to_owned(),
        PathBuf::from("notes.txt"),
        "plain text line",
    );
    editor.format_document();
    assert_eq!(editor.state, EditState::Clean);
    assert_eq!(
        editor.notice.as_deref(),
        Some("Format is available for JSON and XML")
    );

    let mut json_txt = Editor::from_content("a.txt".into(), PathBuf::from("a.txt"), r#"{"a":1}"#);
    json_txt.format_document();
    assert_eq!(json_txt.notice.as_deref(), Some("Formatted JSON"));

    let mut xml_txt = Editor::from_content(
        "a.txt".into(),
        PathBuf::from("a.txt"),
        "<r><e></e><c><![CDATA[raw]]></c><![CDATA[top]]></r>",
    );
    xml_txt.format_document();
    assert_eq!(xml_txt.notice.as_deref(), Some("Formatted XML"));
}
