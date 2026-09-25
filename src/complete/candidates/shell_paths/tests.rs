//! Purpose: When a typed `~` is home syntax, and handing it back unescaped.

use super::*;

#[test]
fn home_syntax_is_decided_by_the_slash() {
    assert!(is_home_syntax("~"));
    assert!(is_home_syntax("~/"));
    assert!(is_home_syntax("~/Documents"));
    assert!(is_home_syntax("~user/mail"));
    assert!(!is_home_syntax("~backup"));
    assert!(!is_home_syntax("plain"));
    assert!(!is_home_syntax(""));
}

#[test]
fn keep_typed_tilde_leaves_untouched_a_match_that_does_not_start_with_a_tilde() {
    // Typing a bare "~" is home syntax, but a fuzzy match on "~" can land on a
    // candidate that merely contains a tilde partway through (e.g. `escape`
    // never touches a non-leading `~`) — there is nothing here to unwrap.
    assert_eq!(keep_typed_tilde("abc~def".to_owned(), true), "abc~def");
}
