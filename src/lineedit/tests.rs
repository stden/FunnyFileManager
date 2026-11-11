//! Purpose: Exercises every cursor, edit, and kill operation of `LineEditor`.
//! Kill and yank are in `tests/kill.rs`, multibyte text in `tests/utf8.rs`.

use super::*;

mod kill;
mod utf8;

/// Build a line with the caret parked at `cursor` bytes from the start.
fn at(text: &str, cursor: usize) -> LineEditor {
    let mut line = LineEditor::new(text);
    line.cursor = cursor;
    line
}

#[test]
fn new_puts_the_caret_at_the_end() {
    let line = LineEditor::new("cd /tmp");
    assert_eq!(line.text(), "cd /tmp");
    assert_eq!(line.cursor().byte(), 7);
    assert!(line.at_end());
}

#[test]
fn default_is_an_empty_line() {
    let line = LineEditor::default();
    assert!(line.is_empty());
    assert_eq!(line.cursor().byte(), 0);
    assert!(line.at_end());
}

#[test]
fn insert_char_splices_at_the_caret() {
    let mut line = at("cd tmp", 3);
    line.insert_char('/');
    assert_eq!(line.text(), "cd /tmp");
    assert_eq!(line.cursor().byte(), 4);
}

#[test]
fn insert_str_advances_by_its_whole_length() {
    let mut line = at("cd tmp", 3);
    line.insert_str("/var/");
    assert_eq!(line.text(), "cd /var/tmp");
    assert_eq!(line.cursor().byte(), 8);
}

#[test]
fn backspace_removes_the_character_before_the_caret() {
    let mut line = at("cdd /tmp", 3);
    line.backspace();
    assert_eq!(line.text(), "cd /tmp");
    assert_eq!(line.cursor().byte(), 2);
}

#[test]
fn backspace_at_the_start_is_a_no_op() {
    let mut line = at("cd", 0);
    line.backspace();
    assert_eq!(line.text(), "cd");
    assert_eq!(line.cursor().byte(), 0);
}

#[test]
fn delete_forward_removes_the_character_under_the_caret() {
    let mut line = at("cdd /tmp", 2);
    line.delete_forward();
    assert_eq!(line.text(), "cd /tmp");
    // The caret does not move: the tail slid left under it.
    assert_eq!(line.cursor().byte(), 2);
}

#[test]
fn delete_forward_at_the_end_is_a_no_op() {
    let mut line = LineEditor::new("cd");
    line.delete_forward();
    assert_eq!(line.text(), "cd");
    assert_eq!(line.cursor().byte(), 2);
}

#[test]
fn left_and_right_saturate_at_the_ends() {
    let mut line = at("ab", 0);
    line.left();
    assert_eq!(line.cursor().byte(), 0);
    line.right();
    line.right();
    assert_eq!(line.cursor().byte(), 2);
    line.right();
    assert_eq!(line.cursor().byte(), 2);
}

#[test]
fn home_and_end_jump_to_the_edges() {
    let mut line = at("cd /tmp", 3);
    line.home();
    assert_eq!(line.cursor().byte(), 0);
    assert!(!line.at_end());
    line.end();
    assert_eq!(line.cursor().byte(), 7);
    assert!(line.at_end());
}

#[test]
fn before_and_after_cursor_split_the_line() {
    let line = at("cd /tmp", 3);
    assert_eq!(line.before_cursor(), "cd ");
    assert_eq!(line.after_cursor(), "/tmp");
}

#[test]
fn set_text_replaces_the_line_and_parks_the_caret_at_the_end() {
    let mut line = at("cd /tmp", 2);
    line.set_text("ls -la");
    assert_eq!(line.text(), "ls -la");
    assert_eq!(line.cursor().byte(), 6);
}

#[test]
fn set_split_leaves_the_caret_between_head_and_tail() {
    let mut line = at("cd /tm/rest", 6);
    line.set_split("cd /tmp/", "/rest");
    assert_eq!(line.text(), "cd /tmp//rest");
    assert_eq!(line.cursor().byte(), 8);
    assert_eq!(line.after_cursor(), "/rest");
}

#[test]
fn clear_empties_the_line_but_keeps_the_kill_slot() {
    let mut line = LineEditor::new("cd /tmp");
    line.kill_to_start();
    line.clear();
    assert!(line.is_empty());
    assert_eq!(line.cursor().byte(), 0);
    // Ctrl+C then Ctrl+Y still pastes what was killed, as in bash.
    line.yank();
    assert_eq!(line.text(), "cd /tmp");
}

// ─── word motion ──────────────────────────────────────────────────────────

#[test]
fn word_left_skips_trailing_whitespace_then_the_word() {
    let mut line = LineEditor::new("cd /usr/local  ");
    line.word_left();
    assert_eq!(line.cursor().byte(), 3);
    line.word_left();
    assert_eq!(line.cursor().byte(), 0);
    // Already at the start: stays put rather than underflowing.
    line.word_left();
    assert_eq!(line.cursor().byte(), 0);
}

#[test]
fn word_right_skips_leading_whitespace_then_the_word() {
    let mut line = at("cd  /usr/local", 2);
    line.word_right();
    assert_eq!(line.cursor().byte(), 14);
    line.word_right();
    assert_eq!(line.cursor().byte(), 14);
}

#[test]
fn word_motion_treats_a_path_as_one_word() {
    // The point of the maximal-run-of-non-whitespace rule: Ctrl+W on a path
    // takes the whole path, not one slash-separated component.
    let mut line = LineEditor::new("cat /a/b/c.txt");
    line.word_left();
    assert_eq!(line.cursor().byte(), 4);
}

#[test]
fn from_str_and_from_string_agree() {
    assert_eq!(
        LineEditor::from("cd /tmp"),
        LineEditor::from(String::from("cd /tmp"))
    );
    assert_eq!(LineEditor::from("cd /tmp").cursor().byte(), 7);
}

mod selection;
