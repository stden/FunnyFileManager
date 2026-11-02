//! Purpose: Pins the quick-search query rules that used to live in the dispatcher.

use super::*;

#[test]
fn a_fresh_search_is_idle() {
    let search = QuickSearch::default();
    assert_eq!(search.query(), None);
    assert!(!search.is_active());
    assert!(!search.should_restart());
}

#[test]
fn pushing_builds_the_query() {
    let mut search = QuickSearch::default();
    assert_eq!(search.push('d'), "d");
    assert_eq!(search.push('o'), "do");
    assert_eq!(search.query(), Some("do"));
    assert!(search.is_active());
}

#[test]
fn a_one_character_query_never_restarts() {
    // Restarting it would search for the same character again, forever.
    let mut search = QuickSearch::default();
    search.push('z');
    assert!(!search.should_restart());
}

#[test]
fn a_longer_query_restarts_from_the_last_character() {
    let mut search = QuickSearch::default();
    search.push('d');
    search.push('o');
    assert!(search.should_restart());
    assert_eq!(search.restart_with('o'), "o");
    assert!(!search.should_restart());
}

#[test]
fn restart_length_is_counted_in_characters_not_bytes() {
    // A single Cyrillic letter is two bytes and a CJK one is three; a
    // byte-length test called either of them a long query and restarted a
    // search that had only just begun.
    let mut search = QuickSearch::default();
    search.push('я');
    assert!(!search.should_restart(), "'я' is one character");

    let mut search = QuickSearch::default();
    search.push('漢');
    assert!(!search.should_restart(), "'漢' is one character");

    let mut search = QuickSearch::default();
    search.push('я');
    search.push('д');
    assert!(search.should_restart());
}

#[test]
fn backspace_shortens_the_query_and_ends_the_search_at_the_last_character() {
    let mut search = QuickSearch::default();
    search.push('a');
    search.push('b');
    assert_eq!(search.backspace(), Some("a"));
    assert_eq!(
        search.backspace(),
        None,
        "the last character ends the search"
    );
    assert_eq!(search, QuickSearch::Idle, "never an empty Active query");
}

#[test]
fn backspace_while_idle_is_harmless() {
    let mut search = QuickSearch::default();
    assert_eq!(search.backspace(), None);
    assert_eq!(search, QuickSearch::Idle);
}

#[test]
fn backspace_removes_a_whole_multibyte_character() {
    let mut search = QuickSearch::default();
    search.push('я');
    search.push('д');
    assert_eq!(search.backspace(), Some("я"));
}

#[test]
fn clear_ends_the_search_from_any_state() {
    let mut search = QuickSearch::default();
    search.push('a');
    search.clear();
    assert_eq!(search, QuickSearch::Idle);
    search.clear();
    assert_eq!(search, QuickSearch::Idle);
}
