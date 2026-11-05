//! Unit tests: TagSpan boundary conditions and cursor-match rules.

use super::{TagSpan};

fn lines(src: &[&str]) -> Vec<String> {
    src.iter().map(|&s| s.to_owned()).collect()
}

#[test]
fn tag_span_new_and_contains_cover_boundary_conditions() {
    let span = TagSpan::new(2, 5, 10);
    assert!(span.contains(2, 5), "start is inclusive");
    assert!(span.contains(2, 9), "last col is inclusive");
    assert!(!span.contains(2, 10), "end col is exclusive");
    assert!(!span.contains(1, 7), "wrong line");
}

#[test]
fn matches_cursor_is_true_when_cursor_is_inside_or_just_past_whitespace_end() {
    let src = lines(&["<div> text"]);
    let span = TagSpan::new(0, 0, 5); // `<div>` columns 0..5
    assert!(span.matches_cursor(&src, 0, 3));
    // At end_col with a space following — trailing-whitespace rule.
    assert!(span.matches_cursor(&src, 0, 5));
    // At end_col with a non-whitespace character following — not a match.
    let src2 = lines(&["<div>x"]);
    assert!(!span.matches_cursor(&src2, 0, 5));
}

#[test]
fn matches_cursor_false_for_unrelated_position() {
    let src = lines(&["<b>hello"]);
    let span = TagSpan::new(0, 0, 3);
    assert!(!span.matches_cursor(&src, 0, 7));
    assert!(!span.matches_cursor(&src, 1, 0));
}
