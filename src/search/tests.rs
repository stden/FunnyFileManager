//! Unit tests: where the next match is, and that the wrap visits every
//! position exactly once.

use super::*;

fn lines(text: &[&str]) -> Vec<String> {
    text.iter().map(|line| (*line).to_owned()).collect()
}

/// A plain search: forwards, ignoring case, matching anywhere.
fn search(query: &str) -> Search {
    Search::new(Needle::new(query).expect("test queries are never empty"))
}

/// A search with one option changed from the default.
fn with(query: &str, change: impl FnOnce(&mut Search)) -> Search {
    let mut search = search(query);
    change(&mut search);
    search
}

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

#[test]
fn case_is_ignored_unless_the_search_says_otherwise() {
    let buffer = lines(&["Hello World"]);
    assert_eq!(
        search("hello").find(&buffer, Caret::new(0, 0)),
        Some(Caret::new(0, 0)),
        "the default search ignores case"
    );

    let exact = with("hello", |s| s.case = Case::Match);
    assert_eq!(exact.find(&buffer, Caret::new(0, 0)), None);
}

/// Whole-word mode must accept a match at the very start and the very end of a
/// line: the ends of the line are boundaries, not word characters.
#[test]
fn whole_words_respects_boundaries_including_the_line_ends() {
    let whole = |query: &str| with(query, |s| s.words = WordMatch::WholeWords);
    let buffer = lines(&["cat concatenate cat_dog cat"]);

    assert_eq!(
        whole("cat").find(&buffer, Caret::new(0, 0)),
        Some(Caret::new(0, 24)),
        "must skip `concatenate` and `cat_dog`, and accept the one at the end"
    );
    // Without the option, the substring inside `concatenate` is a match.
    assert_eq!(
        search("cat").find(&buffer, Caret::new(0, 0)),
        Some(Caret::new(0, 7))
    );
}

#[test]
fn an_underscore_joins_a_word_rather_than_breaking_it() {
    let whole = with("cat", |s| s.words = WordMatch::WholeWords);
    assert_eq!(whole.find(&lines(&["cat_dog"]), Caret::new(0, 0)), None);
}

#[test]
fn a_reverse_search_moves_backwards_from_the_caret() {
    let buffer = lines(&["foo bar foo"]);
    let back = with("foo", |s| s.direction = Direction::Backward);
    assert_eq!(back.find(&buffer, Caret::new(0, 8)), Some(Caret::new(0, 0)));
}

#[test]
fn a_reverse_search_wraps_to_the_end_of_the_buffer() {
    let buffer = lines(&["hay", "hay", "needle"]);
    let back = with("needle", |s| s.direction = Direction::Backward);
    assert_eq!(back.find(&buffer, Caret::new(0, 0)), Some(Caret::new(2, 0)));
}

/// A reverse search must not answer with the match the caret already sits on,
/// or Shift+F7 would never move.
#[test]
fn a_reverse_search_does_not_return_the_caret_position_itself() {
    let buffer = lines(&["foo bar foo"]);
    let back = with("foo", |s| s.direction = Direction::Backward);
    // Standing on the second `foo`, the previous one is at 0 — not 8 again.
    assert_eq!(back.find(&buffer, Caret::new(0, 8)), Some(Caret::new(0, 0)));
    // Standing on the first, it wraps round to the second.
    assert_eq!(back.find(&buffer, Caret::new(0, 0)), Some(Caret::new(0, 8)));
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

// ─── the dialog's ring and its two committing buttons ───────────────────────

/// Backwards round the controls, and a full turn back to where it started.
/// Written against the ring rather than a list of expected pairs, so a control
/// added in the middle does not need this test rewritten.
#[test]
fn the_search_dialog_walks_backwards_and_comes_back_round() {
    use super::SearchField;

    let start = SearchField::default();
    let mut field = start;
    for _ in 0..SearchField::ORDER.len() {
        field = field.previous();
    }
    assert_eq!(field, start, "a full turn backwards");

    assert_eq!(start.next().previous(), start, "one step and back");
}

/// Only the two Find buttons commit to a direction; from anywhere else Enter
/// keeps whatever the dialog already had.
#[test]
fn only_the_find_buttons_commit_to_a_direction() {
    use super::{Direction, SearchField};

    assert_eq!(SearchField::FindNext.direction(), Some(Direction::Forward));
    assert_eq!(
        SearchField::FindPrevious.direction(),
        Some(Direction::Backward)
    );
    for field in [
        SearchField::Query,
        SearchField::Case,
        SearchField::Words,
        SearchField::Cancel,
    ] {
        assert_eq!(field.direction(), None, "{field:?} leaves it alone");
    }
}

/// Toggling `Words` a second time must return to `Anywhere` rather than stick
/// on `WholeWords` — the earlier checkbox test only flips it once.
#[test]
fn toggling_words_twice_returns_to_anywhere() {
    use super::{SearchField, SearchPrompt, WordMatch};

    let mut prompt = SearchPrompt {
        field: SearchField::Words,
        ..SearchPrompt::default()
    };
    assert_eq!(prompt.words, WordMatch::Anywhere);

    prompt.toggle();
    assert_eq!(prompt.words, WordMatch::WholeWords);

    prompt.toggle();
    assert_eq!(
        prompt.words,
        WordMatch::Anywhere,
        "toggling back must return to Anywhere"
    );
}

/// Space flips a checkbox and does nothing anywhere else, which is what keeps
/// it a space in the query field.
#[test]
fn space_flips_the_two_checkboxes_and_nothing_else() {
    use super::{Case, SearchField, SearchPrompt, WordMatch};

    let mut prompt = SearchPrompt::default();
    let (case, words) = (prompt.case, prompt.words);

    prompt.field = SearchField::Case;
    prompt.toggle();
    assert_ne!(prompt.case, case);
    prompt.toggle();
    assert_eq!(prompt.case, case, "and back again");

    prompt.field = SearchField::Words;
    prompt.toggle();
    assert_ne!(prompt.words, words);

    for field in [SearchField::Query, SearchField::Cancel] {
        let before = (prompt.case, prompt.words);
        prompt.field = field;
        prompt.toggle();
        assert_eq!((prompt.case, prompt.words), before, "{field:?}");
    }
    assert_ne!(Case::Match, Case::Ignore);
    assert_ne!(WordMatch::WholeWords, WordMatch::Anywhere);
}
