//! Purpose: the fast path must answer exactly what the allocating expression did.
//!
//! Each property is asserted twice: once as the behaviour callers rely on, and
//! once as agreement with `to_lowercase()`, which is what these functions
//! replaced. The second form is the one that catches a "clever" fast path that
//! is merely close.

use super::{contains_ignore_case, find_ignore_case, folded_name, starts_with_ignore_case};

/// What the old expression computed, kept here as the oracle.
fn slow_contains(haystack: &str, needle: &str) -> bool {
    haystack.to_lowercase().contains(&needle.to_lowercase())
}

fn slow_starts_with(haystack: &str, needle: &str) -> bool {
    haystack.to_lowercase().starts_with(&needle.to_lowercase())
}

fn slow_find(haystack: &str, needle: &str) -> Option<usize> {
    haystack.to_lowercase().find(&needle.to_lowercase())
}

/// Inputs worth disagreeing on: empty strings, a needle longer than the
/// haystack, matches at both ends, mixed case in either operand, and non-ASCII
/// on the fallback path.
const CASES: &[(&str, &str)] = &[
    ("", ""),
    ("", "a"),
    ("a", ""),
    ("Document.txt", "doc"),
    ("Document.txt", "DOC"),
    ("document.txt", "Doc"),
    ("Document.txt", "ment"),
    ("Document.txt", ".TXT"),
    ("Document.txt", "document.txt"),
    ("Document.txt", "document.txt.bak"),
    ("Document.txt", "zzz"),
    ("aaa", "aa"),
    ("service-0001-ssh", "SSH"),
    ("Straße", "STRASSE"),
    ("Straße", "straße"),
    ("ПАПКА", "папка"),
    ("папка", "ПАП"),
    ("İstanbul", "i"),
    ("İstanbul", "stan"),
    ("naïve", "NAÏ"),
    ("ascii", "Ï"),
    ("Hello", "ell"),
    ("Hello", "ELL"),
];

#[test]
fn contains_agrees_with_the_allocating_expression() {
    for (haystack, needle) in CASES {
        assert_eq!(
            contains_ignore_case(haystack, needle),
            slow_contains(haystack, needle),
            "contains_ignore_case({haystack:?}, {needle:?})"
        );
    }
}

#[test]
fn starts_with_agrees_with_the_allocating_expression() {
    for (haystack, needle) in CASES {
        assert_eq!(
            starts_with_ignore_case(haystack, needle),
            slow_starts_with(haystack, needle),
            "starts_with_ignore_case({haystack:?}, {needle:?})"
        );
    }
}

#[test]
fn an_empty_needle_matches_everything() {
    assert!(contains_ignore_case("anything", ""));
    assert!(starts_with_ignore_case("anything", ""));
    assert!(contains_ignore_case("", ""));
    assert!(starts_with_ignore_case("", ""));
}

#[test]
fn a_needle_longer_than_the_haystack_never_matches() {
    assert!(!contains_ignore_case("ab", "abc"));
    assert!(!starts_with_ignore_case("ab", "abc"));
}

#[test]
fn case_is_ignored_in_both_operands() {
    assert!(contains_ignore_case("ReadMe.MD", "readme"));
    assert!(contains_ignore_case("readme.md", "ReadMe"));
    assert!(starts_with_ignore_case("ReadMe.MD", "readme"));
    assert!(starts_with_ignore_case("readme.md", "ReadMe"));
}

/// `starts_with` must not accept a match that merely occurs somewhere.
#[test]
fn starts_with_is_anchored() {
    assert!(contains_ignore_case("Document.txt", "txt"));
    assert!(!starts_with_ignore_case("Document.txt", "txt"));
}

/// The index stays in the folded string. `İ` expands, so it is not the byte
/// offset of the same letters in the original haystack.
#[test]
fn find_agrees_with_the_allocating_expression() {
    for (haystack, needle) in CASES {
        assert_eq!(
            find_ignore_case(haystack, needle),
            slow_find(haystack, needle),
            "find_ignore_case({haystack:?}, {needle:?})"
        );
    }
    assert_eq!(find_ignore_case("Hello", "ell"), Some(1));
    assert_eq!(find_ignore_case("", ""), Some(0));
    let expanded = find_ignore_case("İstanbul", "stan");
    assert_eq!(expanded, slow_find("İstanbul", "stan"));
    assert_ne!(expanded, "İstanbul".find("stan"));
}

/// Prefix search binary-searches folded names, so the fold must agree with
/// `starts_with_ignore_case` on every case that function already locks.
#[test]
fn folded_names_agree_with_starts_with_ignore_case() {
    for (haystack, needle) in CASES {
        assert_eq!(
            folded_name(haystack).starts_with(&folded_name(needle)),
            starts_with_ignore_case(haystack, needle),
            "folded prefix {haystack:?} / {needle:?}"
        );
    }
}

#[test]
fn ascii_anchor_rank_and_probe_prefer_punctuation_and_match_both_cases() {
    use super::{ascii_anchor_rank, has_ascii_anchor};
    assert!(ascii_anchor_rank(b'_') < ascii_anchor_rank(b'7'));
    assert!(ascii_anchor_rank(b'7') < ascii_anchor_rank(b'z'));
    assert!(ascii_anchor_rank(b'z') < ascii_anchor_rank(b'e'));
    assert!(ascii_anchor_rank(b'e') < ascii_anchor_rank(b' '));
    assert!(has_ascii_anchor(b"Hello_World", b'_'));
    assert!(has_ascii_anchor(b"Hello_World", b'w'));
    assert!(!has_ascii_anchor(b"Hello_World", b'q'));
}
