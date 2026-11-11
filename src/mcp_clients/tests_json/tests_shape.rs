//! Purpose: JSON configs FFM cannot use: empty files, files that do not
//! parse, and keys that are not objects, which are never rewritten.

use super::*;

#[test]
fn empty_whitespace_and_bom_only_files_are_not_configured() {
    for text in ["", "  \n\t", "\u{feff}", "\u{feff}\n"] {
        assert_eq!(
            json_state(text, &all_projects()),
            ConfigState::NotConfigured
        );
    }
}

#[test]
fn a_file_that_does_not_parse_names_the_line_and_column() {
    let ConfigState::Blocked(Blocker::Unparsable(message)) =
        json_state("{\n  \"a\": 1,\n", &all_projects())
    else {
        panic!("expected a parse error");
    };
    assert!(message.contains("line"), "{message}");
    assert!(message.contains("column"), "{message}");
}

#[test]
fn unexpected_shapes_name_the_key_that_is_not_an_object() {
    let cases = [
        ("[]", "the top level"),
        (r#"{"mcpServers": "x"}"#, "mcpServers"),
        (r#"{"mcpServers": {"ffm-mcp": []}}"#, "mcpServers.ffm-mcp"),
        (
            r#"{"mcpServers": {"ffm-mcp": {"env": 3}}}"#,
            "mcpServers.ffm-mcp.env",
        ),
    ];
    for (text, key) in cases {
        let blocked = ConfigState::Blocked(Blocker::UnexpectedLayout(key));
        assert_eq!(json_state(text, &all_projects()), blocked, "{text}");
        assert_eq!(
            upsert(ConfigFormat::Json, text, &all_projects()),
            Err(Blocker::UnexpectedLayout(key)),
            "{text}"
        );
    }
}

#[test]
fn a_file_that_does_not_parse_is_never_rewritten() {
    assert!(matches!(
        upsert(ConfigFormat::Json, "{ broken", &all_projects()),
        Err(Blocker::Unparsable(_))
    ));
}
