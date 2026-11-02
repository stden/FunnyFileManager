//! Purpose: Shift-selection over a `LineEditor`: extend and shrink, replace
//! by typing, delete, cut, and the moves and edits that drop it.

use super::*;

#[test]
fn shift_left_selects_back_from_the_caret_and_shift_right_shrinks_it() {
    let mut line = LineEditor::new("report.txt");
    line.extend(LineEditor::left);
    line.extend(LineEditor::left);
    line.extend(LineEditor::left);
    assert_eq!(line.selected_text(), Some("txt"));
    line.extend(LineEditor::right);
    assert_eq!(line.selected_text(), Some("xt"));
}

#[test]
fn shift_home_selects_to_the_start_and_select_all_takes_everything() {
    let mut line = at("hello world", 5);
    line.extend(LineEditor::home);
    assert_eq!(line.selected_text(), Some("hello"));
    line.select_all();
    assert_eq!(line.selected_text(), Some("hello world"));
    assert_eq!(line.cursor().byte(), 11);
}

#[test]
fn typing_replaces_the_selection() {
    let mut line = LineEditor::new("old name.txt");
    line.select_all();
    line.insert_char('n');
    assert_eq!(line.text(), "n");
    assert_eq!(line.selection(), None);

    let mut line = LineEditor::new("a-b-c");
    line.extend(LineEditor::word_left);
    line.insert_str("xyz");
    assert_eq!(line.text(), "xyz");
}

#[test]
fn backspace_and_delete_remove_the_selection_alone() {
    let mut line = at("abcdef", 2);
    line.extend(LineEditor::right);
    line.extend(LineEditor::right);
    line.backspace();
    assert_eq!((line.text(), line.cursor().byte()), ("abef", 2));

    let mut line = at("abcdef", 4);
    line.extend(LineEditor::left);
    line.delete_forward();
    assert_eq!((line.text(), line.cursor().byte()), ("abcef", 3));
}

#[test]
fn cut_hands_back_the_selection_and_removes_it() {
    let mut line = LineEditor::new("keep cut");
    line.extend(LineEditor::word_left);
    assert_eq!(line.cut().as_deref(), Some("cut"));
    assert_eq!(line.text(), "keep ");
    assert_eq!(line.cut(), None, "nothing selected, nothing cut");
}

#[test]
fn a_plain_move_or_a_new_text_drops_the_selection() {
    let mut line = LineEditor::new("abc");
    line.select_all();
    line.left();
    assert_eq!(line.selection(), None);
    line.select_all();
    line.set_text("x");
    assert_eq!(line.selection(), None);
    line.select_all();
    line.clear();
    assert_eq!(line.selection(), None);
}

#[test]
fn a_selection_never_splits_a_character() {
    let mut line = LineEditor::new("日本語");
    line.extend(LineEditor::left);
    assert_eq!(line.selected_text(), Some("語"));
    line.backspace();
    assert_eq!(line.text(), "日本");
}

#[test]
fn an_empty_selection_is_none() {
    let mut line = LineEditor::new("");
    line.select_all();
    assert_eq!(line.selection(), None);
    assert!(!line.delete_selection());
}
