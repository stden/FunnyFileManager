//! Purpose: Exercises every cursor, edit, and kill operation of `LineEditor`.

use super::*;

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

// ─── kill and yank ────────────────────────────────────────────────────────

#[test]
fn kill_word_left_cuts_the_word_before_the_caret() {
    let mut line = LineEditor::new("cat /a/b/c.txt");
    line.kill_word_left();
    assert_eq!(line.text(), "cat ");
    assert_eq!(line.cursor().byte(), 4);
    line.yank();
    assert_eq!(line.text(), "cat /a/b/c.txt");
}

#[test]
fn kill_to_start_cuts_everything_before_the_caret() {
    let mut line = at("cd /tmp", 3);
    line.kill_to_start();
    assert_eq!(line.text(), "/tmp");
    assert_eq!(line.cursor().byte(), 0);
    line.end();
    line.yank();
    assert_eq!(line.text(), "/tmpcd ");
}

#[test]
fn kill_to_end_cuts_everything_from_the_caret() {
    let mut line = at("cd /tmp", 3);
    line.kill_to_end();
    assert_eq!(line.text(), "cd ");
    assert_eq!(line.cursor().byte(), 3);
}

#[test]
fn an_empty_kill_leaves_the_previous_one_intact() {
    let mut line = LineEditor::new("cd /tmp");
    line.kill_to_start();
    assert!(line.is_empty());
    // Ctrl+K at the end of an empty line must not erase what Ctrl+U captured.
    line.kill_to_end();
    line.yank();
    assert_eq!(line.text(), "cd /tmp");
}

#[test]
fn yank_without_a_kill_is_a_no_op() {
    let mut line = LineEditor::new("cd");
    line.yank();
    assert_eq!(line.text(), "cd");
    assert_eq!(line.cursor().byte(), 2);
}

#[test]
fn yank_repeats_rather_than_consuming_the_slot() {
    let mut line = LineEditor::new("ab");
    line.kill_to_start();
    line.yank();
    line.yank();
    assert_eq!(line.text(), "abab");
}

// ─── UTF-8 ────────────────────────────────────────────────────────────────

#[test]
fn cursor_moves_whole_characters_through_multibyte_text() {
    // Cyrillic (2 bytes), CJK (3 bytes), emoji (4 bytes) — a byte-stepping
    // cursor would land mid-character and panic on the next slice.
    // я(2) д(2) 漢(3) 🙂(4) = 11 bytes.
    let mut line = LineEditor::new("яд漢🙂");
    assert_eq!(line.cursor().byte(), 11);
    line.left();
    assert_eq!(line.cursor().byte(), 7);
    line.left();
    assert_eq!(line.cursor().byte(), 4);
    line.left();
    assert_eq!(line.cursor().byte(), 2);
    line.left();
    assert_eq!(line.cursor().byte(), 0);
    line.right();
    assert_eq!(line.cursor().byte(), 2);
}

#[test]
fn backspace_removes_a_whole_multibyte_character() {
    let mut line = LineEditor::new("cd 漢字");
    line.backspace();
    assert_eq!(line.text(), "cd 漢");
    assert_eq!(line.cursor().byte(), 6);
}

#[test]
fn insert_into_multibyte_text_keeps_the_caret_on_a_boundary() {
    let mut line = at("漢字", 3);
    line.insert_char('x');
    assert_eq!(line.text(), "漢x字");
    assert_eq!(line.cursor().byte(), 4);
    line.right();
    assert!(line.at_end());
}

#[test]
fn word_motion_survives_non_ascii_whitespace() {
    // U+3000 IDEOGRAPHIC SPACE is 3 bytes and is `char::is_whitespace`;
    // advancing past it by one byte would split it.
    let mut line = LineEditor::new("ab\u{3000}cd");
    line.word_left();
    assert_eq!(line.cursor().byte(), 5);
    assert_eq!(line.after_cursor(), "cd");
}

#[test]
fn from_str_and_from_string_agree() {
    assert_eq!(
        LineEditor::from("cd /tmp"),
        LineEditor::from(String::from("cd /tmp"))
    );
    assert_eq!(LineEditor::from("cd /tmp").cursor().byte(), 7);
}
