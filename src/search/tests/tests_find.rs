//! Purpose: Where the next match is — same line, later lines, the wrap, and character columns.

use super::*;

#[test]
fn finds_the_next_match_after_the_caret_on_the_same_line() {
    let buffer = lines(&["foo bar foo"]);
    let found = search("foo").find(&buffer, Caret::new(0, 0));
    assert_eq!(found, Some(Caret::new(0, 8)), "must move past the caret");
}

#[test]
fn finds_a_match_on_a_later_line() {
    let buffer = lines(&["alpha", "beta", "gamma"]);
    assert_eq!(
        search("gamma").find(&buffer, Caret::new(0, 0)),
        Some(Caret::new(2, 0))
    );
}

/// The search wraps rather than stopping at the end of the buffer: answering
/// "continue from the top" is what the user wants every time.
#[test]
fn wraps_past_the_end_of_the_buffer() {
    let buffer = lines(&["needle", "hay", "hay"]);
    assert_eq!(
        search("needle").find(&buffer, Caret::new(2, 0)),
        Some(Caret::new(0, 0))
    );
}

/// …and back to the near side of the starting line, which is the last place
/// left to look.
#[test]
fn wraps_back_onto_the_starting_line() {
    let buffer = lines(&["needle and hay"]);
    assert_eq!(
        search("needle").find(&buffer, Caret::new(0, 5)),
        Some(Caret::new(0, 0))
    );
}

/// A search with no match must terminate rather than circle the buffer.
#[test]
fn a_missing_string_is_reported_rather_than_looped_over() {
    let buffer = lines(&["alpha", "beta", "gamma"]);
    assert_eq!(search("zeta").find(&buffer, Caret::new(1, 2)), None);
}

/// An empty query is unrepresentable rather than merely fruitless: `Needle`
/// refuses to build one, so no caller has to remember to check.
#[test]
fn an_empty_query_cannot_be_built_and_an_empty_buffer_finds_nothing() {
    assert_eq!(Needle::new(""), None);
    assert_eq!(search("text").find(&[], Caret::default()), None);
}

/// Repeating the search must walk the matches rather than stick on one.
#[test]
fn repeating_the_search_walks_every_match_and_returns_to_the_first() {
    let buffer = lines(&["a", "match", "b", "match"]);
    let query = search("match");

    let first = query.find(&buffer, Caret::new(0, 0)).unwrap();
    assert_eq!(first, Caret::new(1, 0));
    let second = query.find(&buffer, first).unwrap();
    assert_eq!(second, Caret::new(3, 0));
    let third = query.find(&buffer, second).unwrap();
    assert_eq!(third, first, "wraps back to the first match");
}

/// Columns are character indices, so a match after a multi-byte character must
/// report where the caret goes, not how many bytes precede it.
#[test]
fn columns_count_characters_not_bytes() {
    let buffer = lines(&["привет мир"]);
    assert_eq!(
        search("мир").find(&buffer, Caret::new(0, 0)),
        Some(Caret::new(0, 7)),
        "seven characters precede `мир`, though many more bytes do"
    );
}

#[test]
fn a_needle_longer_than_the_line_is_not_a_match() {
    let buffer = lines(&["ab", "abcdef"]);
    assert_eq!(
        search("abcdef").find(&buffer, Caret::new(0, 0)),
        Some(Caret::new(1, 0)),
        "the short line must be skipped, not panic on the subtraction"
    );
}

/// `matches_at` must refuse a position the needle would run past rather than
/// panic on the out-of-bounds slice. `find` never actually reaches it —
/// `last_start` already keeps every column it tries in range — but the guard
/// is the only thing standing between a future caller and that panic, so it
/// is pinned directly.
#[test]
fn matches_at_refuses_a_position_the_needle_would_run_past() {
    let query = search("needle");
    let haystack: Vec<char> = "hay".chars().collect();
    let needle: Vec<char> = "needle".chars().collect();
    assert!(!query.matches_at(&haystack, &needle, 0));
}

/// An empty line has no columns to try; the scan must step over it.
#[test]
fn empty_lines_are_stepped_over() {
    let buffer = lines(&["", "", "found", ""]);
    assert_eq!(
        search("found").find(&buffer, Caret::new(0, 0)),
        Some(Caret::new(2, 0))
    );
}
