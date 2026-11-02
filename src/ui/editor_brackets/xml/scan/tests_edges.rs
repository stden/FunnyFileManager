//! Purpose: The scanner's edges: a cursor in text between two tags reports no
//! pair, and a step taken outside an open tag only moves past its character.

use super::*;

#[test]
fn a_cursor_in_text_between_tags_reports_no_pair() {
    let lines = ["<a>", "text", "</a>"].map(str::to_owned);
    assert!(XmlScanner::default().find_pair(&lines, 1, 0).is_none());
}

#[test]
fn a_step_outside_an_open_tag_only_moves_past_its_character() {
    let mut scanner = XmlScanner::default();
    let (advance, columns, pair) = scanner.step_open_tag(0, 0, "x");
    assert_eq!((advance, columns), (1, 1));
    assert!(pair.is_none());
}
