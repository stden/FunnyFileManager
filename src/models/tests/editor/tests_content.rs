//! Purpose: unit tests for the editor's content round trip and line endings.

use super::*;

#[test]
fn test_editor_from_content_round_trip() {
    // CRLF files must survive untouched: F4 + F2 on a .bat or a CRLF-committed
    // source file used to rewrite every line ending and produce a whole-file diff.
    for content in [
        "",
        "a",
        "a\n",
        "\n",
        "a\nb",
        "a\nb\n",
        "a\r\nb\r\n",
        "a\r\n",
    ] {
        let editor = editor_from(content);
        assert_eq!(editor.content(), content, "round trip of {content:?}");
        assert!(!editor.state.is_modified());
    }
}

#[test]
fn test_editor_keeps_crlf_while_editing() {
    let mut editor = editor_from("alpha\r\nbeta\r\n");
    assert_eq!(editor.format.line_ending, LineEnding::Crlf);
    editor.move_end();
    editor.insert_newline();
    assert_eq!(editor.content(), "alpha\r\n\r\nbeta\r\n");
}

#[test]
fn test_editor_is_crlf_only_when_every_line_is() {
    // A single stray CRLF must not convert an otherwise LF file, and a single
    // LF must not convert a CRLF-majority one either: a mixed file is split
    // on LF, and each CRLF line keeps its CR as text.
    assert_eq!(
        editor_from("a\r\nb\nc\nd\n").format.line_ending,
        LineEnding::Lf
    );
    assert_eq!(
        editor_from("a\r\nb\r\nc\n").format.line_ending,
        LineEnding::Lf
    );
    assert_eq!(
        editor_from("a\r\nb\r\n").format.line_ending,
        LineEnding::Crlf
    );
}

#[test]
fn test_editor_empty_file_has_one_line() {
    let editor = editor_from("");
    assert_eq!(editor.lines, vec![String::new()]);
}
