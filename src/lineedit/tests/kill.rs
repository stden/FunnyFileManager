//! Purpose: Exercises the kill slot: what each kill cuts, and what a yank
//! pastes back.

use super::*;

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
