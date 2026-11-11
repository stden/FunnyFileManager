//! Purpose: the fast path must answer exactly what the allocating expression did.
//!
//! Each property is asserted twice: once as the behaviour callers rely on, and
//! once as agreement with `to_lowercase()`, which is what these functions
//! replaced. The second form is the one that catches a "clever" fast path that
//! is merely close.

use super::{
    contains_ignore_case, eq_ignore_case, find_ignore_case, folded_name, starts_with_ignore_case,
    CaseNeedle,
};

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

/// Inputs worth disagreeing on: empty strings, short & SIMD-length haystacks,
/// matches at both ends, mixed case in either operand, and non-ASCII hazards.
const CASES: &[(&str, &str)] = &[
    ("", ""),
    ("", "a"),
    ("a", ""),
    ("Document.txt", "doc"),
    ("Document.txt", "DOC"),
    ("document.txt", "Doc"),
    ("Document.txt", "ment"),
    ("Document.txt", ".TXT"),
    ("Document.txt.", "TXT."),
    ("Document.txt", "document.txt"),
    ("Document.txt", "document.txt.bak"),
    ("Document.txt", "zzz"),
    ("Document.txt", "file_i"),
    ("aaa", "aa"),
    ("service-0001-ssh", "SSH"),
    ("service-0001-ssh", "0001"),
    ("service-0001-ssh", "servic"),
    ("0123456789012345678901234567890123456789_World", "WORLD"),
    ("0123456789012345678901234567890123456789_World", "missing"),
    ("0123456789012345678901234567890123456789_42_ok", "_42"),
    ("0123456789012345678901234567890123456789_42_ok", "_99"),
    ("Straße", "STRASSE"),
    ("Straße", "straße"),
    ("ПАПКА", "папка"),
    ("папка", "ПАП"),
    ("ПАПКА_readme.txt", "README"),
    ("ПАПКА_отчёт.txt", "README"),
    ("Readme_отчёт.txt", "READ"),
    ("ПАПКА_kelvin", "kelvin"),
    ("ПАПКА", "kelvin"),
    ("ПАПКА", "ї_z"),
    ("ПАПКА", "ї"),
    ("ẞ_test", "TEST"),
    ("İstanbul", "i"),
    ("İstanbul", "stan"),
    ("\u{212a}elvin", "kelvin"),
    ("kelvin", "\u{212a}elvin"),
    ("\u{212a}elvin", "k"),
    ("naïve", "NAÏ"),
    ("ascii", "Ï"),
    ("Hello", "ell"),
    ("Hello", "ELL"),
];

#[test]
fn contains_and_case_needle_agree_with_the_allocating_expression() {
    for (haystack, needle) in CASES {
        let expected = slow_contains(haystack, needle);
        assert_eq!(
            contains_ignore_case(haystack, needle),
            expected,
            "contains_ignore_case({haystack:?}, {needle:?})"
        );
        assert_eq!(
            CaseNeedle::new(needle).contains_in(haystack),
            expected,
            "CaseNeedle::new({needle:?}).contains_in({haystack:?})"
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
fn edge_cases_empty_oversize_case_and_anchoring() {
    assert!(contains_ignore_case("anything", ""));
    assert!(starts_with_ignore_case("anything", ""));
    assert!(contains_ignore_case("", ""));
    assert!(starts_with_ignore_case("", ""));
    assert!(!contains_ignore_case("ab", "abc"));
    assert!(!starts_with_ignore_case("ab", "abc"));
    assert!(contains_ignore_case("ReadMe.MD", "readme"));
    assert!(contains_ignore_case("readme.md", "ReadMe"));
    assert!(starts_with_ignore_case("ReadMe.MD", "readme"));
    assert!(starts_with_ignore_case("readme.md", "ReadMe"));
    assert!(contains_ignore_case("Document.txt", "txt"));
    assert!(!starts_with_ignore_case("Document.txt", "txt"));
}

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

#[test]
fn eq_ignore_case_agrees_with_to_lowercase_across_ascii_and_unicode() {
    for (haystack, needle) in CASES {
        assert_eq!(
            eq_ignore_case(haystack, needle),
            haystack.to_lowercase() == needle.to_lowercase(),
            "eq_ignore_case({haystack:?}, {needle:?})"
        );
    }
    assert!(eq_ignore_case("İstanbul", "i\u{307}STANBUL"));
    assert!(eq_ignore_case("\u{212a}elvin", "KELVIN"));
}

#[test]
fn ascii_safe_helpers_reject_turkic_and_kelvin_hazards_and_locate_matches() {
    use super::{
        find_ascii_ignore_case, find_ascii_with_anchor, is_ascii_safe_needle, rarest_ascii_anchor,
        safe_unicode_ascii_anchor,
    };
    assert!(is_ascii_safe_needle(b"readme_42"));
    assert!(!is_ascii_safe_needle(b"file_i"));
    assert!(!is_ascii_safe_needle(b"kelvin"));
    assert!(!is_ascii_safe_needle("ї".as_bytes()));
    assert_eq!(find_ascii_ignore_case(b"Hello_World", b""), None);
    assert_eq!(find_ascii_ignore_case(&[b'a'; 40], b""), None);
    assert_eq!(find_ascii_ignore_case(b"Hello_World", b"WORLD"), Some(6));
    assert_eq!(
        find_ascii_with_anchor(b"Hello_World", b"WORLD", 0, b'w'),
        Some(6)
    );
    assert_eq!(rarest_ascii_anchor(b"a_b"), Some((1, b'_')));
    assert_eq!(find_ascii_ignore_case(b"Hi", b"longer"), None);
    assert_eq!(safe_unicode_ascii_anchor(b"ik"), None);
    assert_eq!(safe_unicode_ascii_anchor(b"i_k"), Some(b'_'));
}
