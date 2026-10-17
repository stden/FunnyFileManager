//! Purpose: The search options — case, whole words, and a backward search.

use super::*;

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

    let back_exact = with("foo", |s| {
        s.direction = Direction::Backward;
        s.case = Case::Match;
        s.words = WordMatch::WholeWords;
    });
    assert_eq!(
        back_exact.find(&lines(&["foo foobar foo"]), Caret::new(0, 11)),
        Some(Caret::new(0, 0))
    );
}

#[test]
fn unicode_search_supports_exact_case_whole_words_and_backward_direction() {
    let buffer = lines(&["кот котик кот_пес кот"]);
    let exact_miss = with("КОТ", |s| s.case = Case::Match);
    assert_eq!(exact_miss.find(&buffer, Caret::new(0, 0)), None);

    let exact_whole = with("кот", |s| {
        s.case = Case::Match;
        s.words = WordMatch::WholeWords;
    });
    assert_eq!(
        exact_whole.find(&buffer, Caret::new(0, 0)),
        Some(Caret::new(0, 18))
    );

    let back_unicode = with("кот", |s| {
        s.case = Case::Match;
        s.words = WordMatch::WholeWords;
        s.direction = Direction::Backward;
    });
    assert_eq!(
        back_unicode.find(&buffer, Caret::new(0, 18)),
        Some(Caret::new(0, 0))
    );
}
