//! Unit tests: the editor buffer reads a file's lines and writes them back
//! byte for byte, whatever mix of line endings the file uses.

use std::path::PathBuf;

use super::{Editor, LineEnding};

fn open(content: &str) -> Editor {
    Editor::from_content("t.txt".to_owned(), PathBuf::from("t.txt"), content)
}

#[test]
fn mouse_position_uses_display_cells_clamps_and_clears_selection() {
    let mut editor = open("\t界x\nlast");
    editor.select_all();
    editor.jump_to_screen_position(0, 1);
    assert_eq!((editor.cursor_line, editor.cursor_col), (0, 0));
    assert_eq!(editor.selection, super::EditorSelection::None);
    let tab = editor.tab_size.columns();
    editor.jump_to_screen_position(0, tab + 1);
    assert_eq!((editor.cursor_line, editor.cursor_col), (0, 1));
    editor.jump_to_screen_position(usize::MAX, usize::MAX);
    assert_eq!((editor.cursor_line, editor.cursor_col), (1, 4));
}

/// A combining mark takes no cell, and a cursor past the last line scrolls
/// vertically while its missing line puts it at the left edge.
#[test]
fn viewport_counts_tabs_wide_characters_and_combining_marks_in_cells() {
    let mut editor = open("\t界\u{301}x");
    editor.cursor_col = 4;
    assert_eq!(editor.viewport(25, 4), (0, editor.tab_size.columns()));
    editor.cursor_line = 9;
    assert_eq!(editor.viewport(3, 4), (8, 0));
}

#[test]
fn backspace_with_a_stale_public_cursor_does_not_modify_the_document() {
    let mut editor = open("kept");
    editor.cursor_line = 9;
    editor.cursor_col = 1;
    editor.backspace();
    assert_eq!(editor.content(), "kept");
    assert_eq!(editor.state, super::EditState::Clean);
}

/// Each line keeps its own terminator. The buffer used to pick one ending for
/// the whole file by majority and write it after every line, so `one\r\n`
/// lost its CR in an LF-majority file and `c\n` gained one in a CRLF-majority
/// file, although nothing was edited.
#[test]
fn every_line_keeps_its_own_ending_through_a_round_trip() {
    for content in [
        "one\r\ntwo\nthree\n",
        "a\r\nb\r\nc\n",
        "a\nb\nc\r\n",
        "bare\rcarriage\rreturns\n",
        "trailing cr\r",
        "x\r\n\r\n\ny",
        "\r\n",
        "\n\n",
        "no newline at all",
        "",
    ] {
        assert_eq!(open(content).content(), content, "{content:?}");
    }
}

/// A file whose every line ends in CRLF is edited without the CRs, and
/// written back with them: the common Windows file stays clean in the buffer.
#[test]
fn a_file_that_is_all_crlf_is_edited_without_the_carriage_returns() {
    let editor = open("a\r\nb\r\n");

    assert_eq!(editor.lines, ["a", "b"]);
    assert_eq!(editor.format.line_ending, LineEnding::Crlf);
    assert_eq!(editor.content(), "a\r\nb\r\n");
}

/// In a mixed file the CR of a CRLF line stays part of that line, the way Vim
/// keeps it, so a line typed into the file gets the plain LF around it and
/// the untouched lines keep what they had.
#[test]
fn a_new_line_in_a_mixed_file_leaves_the_other_endings_alone() {
    let mut editor = open("one\r\ntwo\n");
    assert_eq!(editor.lines, ["one\r", "two"]);

    editor.move_down(1);
    editor.move_end();
    editor.insert_newline();
    editor.insert_char('3');

    assert_eq!(editor.content(), "one\r\ntwo\n3\n");
}

#[test]
fn cursor_column_move_home_viewport_and_empty_backspace_stay_consistent() {
    let mut editor = open("\t界x");
    editor.move_cursor(super::EditorMotion::Right, super::SelectionMode::Extend);
    editor.move_cursor(super::EditorMotion::Right, super::SelectionMode::Extend);
    editor.move_cursor(super::EditorMotion::Right, super::SelectionMode::Extend);
    assert_eq!(editor.cursor_column(), super::CharColumn::new(3));
    assert_eq!(editor.viewport(10, 4), (0, 4));
    editor.move_home();
    assert_eq!(editor.cursor_column().chars(), 0);
    assert_eq!(editor.selection, super::EditorSelection::None);

    let mut empty = Editor {
        cursor_col: 1,
        ..Default::default()
    };

    empty.backspace();
    assert!(!empty.state.is_modified());
}
