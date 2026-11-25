//! Purpose: Exercises the caret through multibyte text: whole characters only,
//! whatever their width in bytes, whitespace included.

use super::*;

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
