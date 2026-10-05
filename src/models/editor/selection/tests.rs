//! Selection direction, character boundaries, and buffer replacement regressions.

use std::path::PathBuf;

use super::*;
use crate::models::EditState;

fn editor(content: &str) -> Editor {
    Editor::from_content("text".to_owned(), PathBuf::from("text.txt"), content)
}

fn extend(editor: &mut Editor, motion: EditorMotion) {
    editor.move_cursor(motion, SelectionMode::Extend);
}

#[test]
fn extending_select_all_preserves_its_start_without_changing_the_document() {
    let mut editor = editor("first\nlast");
    editor.select_all();
    assert_eq!(
        editor.selection_range(),
        Some(Caret::new(0, 0)..Caret::new(1, 4))
    );
    extend(&mut editor, EditorMotion::Left);
    assert_eq!(editor.selected_text().as_deref(), Some("first\nlas"));
    assert_eq!(editor.state, EditState::Clean);
}

#[test]
fn deleting_an_empty_selection_keeps_the_editor_clean() {
    let mut editor = Editor::default();
    editor.select_all();
    assert_eq!(editor.selection_range(), None);
    assert!(!editor.delete_selection());
    assert_eq!(editor.lines, [""]);
    assert_eq!(editor.state, EditState::Clean);
    assert_eq!(editor.caret(), Caret::new(0, 0));
}

#[test]
fn shift_motion_reduces_and_reverses_around_the_original_anchor() {
    let mut editor = editor("a界🙂z");
    editor.cursor_col = 1;
    extend(&mut editor, EditorMotion::Right);
    extend(&mut editor, EditorMotion::Right);
    assert_eq!(editor.selected_text().as_deref(), Some("界🙂"));
    extend(&mut editor, EditorMotion::Left);
    assert_eq!(editor.selected_text().as_deref(), Some("界"));
    extend(&mut editor, EditorMotion::Left);
    assert_eq!(editor.selected_text(), None);
    extend(&mut editor, EditorMotion::Left);
    assert_eq!(editor.selected_text().as_deref(), Some("a"));
    assert_eq!(
        editor.selection_range(),
        Some(Caret::new(0, 0)..Caret::new(0, 1))
    );
    assert_eq!(editor.state, EditState::Clean);
}

#[test]
fn reverse_multiline_unicode_selection_preserves_crlf_and_replaces_as_one_range() {
    let mut editor = editor("a界\r\nβ🙂\r\nz!");
    editor.cursor_line = 2;
    editor.cursor_col = 1;
    extend(&mut editor, EditorMotion::Up(2));
    assert_eq!(editor.selected_text().as_deref(), Some("界\r\nβ🙂\r\nz"));
    editor.insert_text("X\r\nЖ");
    assert_eq!(editor.lines, ["aX", "Ж!"]);
    assert_eq!(editor.caret(), Caret::new(1, 1));
    assert_eq!(editor.content(), "aX\r\nЖ!");
    assert_eq!(editor.selection, EditorSelection::None);
    assert_eq!(editor.state, EditState::Modified);
}

#[test]
fn typing_newline_and_deletions_replace_the_selection() {
    for edit in [Editor::backspace, Editor::delete_forward] {
        let mut editor = editor("a界\n🙂z");
        editor.cursor_col = 1;
        extend(&mut editor, EditorMotion::Down(1));
        edit(&mut editor);
        assert_eq!(editor.lines, ["az"]);
        assert_eq!(editor.caret(), Caret::new(0, 1));
    }
    let mut typed = editor("a界z");
    typed.cursor_col = 1;
    extend(&mut typed, EditorMotion::Right);
    typed.insert_char('Ж');
    assert_eq!(typed.lines, ["aЖz"]);
    extend(&mut typed, EditorMotion::Left);
    typed.insert_newline();
    assert_eq!(typed.lines, ["a", "z"]);
}

#[test]
fn plain_movement_clears_selection_and_boundaries_remain_valid() {
    for motion in [
        EditorMotion::Left,
        EditorMotion::Right,
        EditorMotion::Up(1),
        EditorMotion::Down(1),
        EditorMotion::Home,
        EditorMotion::End,
    ] {
        let mut editor = editor("abc\nx\ndef");
        extend(&mut editor, EditorMotion::Right);
        editor.move_cursor(motion, SelectionMode::Move);
        assert_eq!(editor.selection, EditorSelection::None);
    }
    let mut editor = editor("界\n");
    extend(&mut editor, EditorMotion::Left);
    assert_eq!(editor.selected_text(), None);
    extend(&mut editor, EditorMotion::Down(usize::MAX));
    extend(&mut editor, EditorMotion::End);
    assert_eq!(editor.selected_text().as_deref(), Some("界"));
    editor.insert_text("");
    assert_eq!(editor.selected_text().as_deref(), Some("界"));
}

#[test]
fn paste_preserves_empty_lines_and_trailing_line_breaks() {
    let mut editor = Editor::default();
    editor.insert_text("界\r\n\r\n🙂\n");
    assert_eq!(editor.lines, ["界", "", "🙂", ""]);
    assert_eq!(editor.caret(), Caret::new(3, 0));
}

#[test]
fn a_large_multiline_paste_preserves_the_document_tail_and_caret() {
    let mut editor = editor(&format!("a界z\r\n{}", "untouched\r\n".repeat(10_000)));
    editor.cursor_col = 1;
    extend(&mut editor, EditorMotion::Right);
    let text = "β\t🙂\r\n".repeat(10_000);
    editor.insert_text(&text);
    assert_eq!(editor.lines[0], "aβ\t🙂");
    assert_eq!(editor.lines[9_999], "β\t🙂");
    assert_eq!(editor.lines[10_000], "z");
    assert_eq!(editor.caret(), Caret::new(10_000, 0));
    assert_eq!(
        editor.content(),
        format!("a{text}z\r\n{}", "untouched\r\n".repeat(10_000))
    );
    assert_eq!(editor.selection, EditorSelection::None);
}
