//! Purpose: Pins the quoting rules completion and `sh` must agree on; cmd's are in `tests_windows.rs`.

use super::*;
use crate::complete::PathSyntax::Posix;

// ─── last_word ────────────────────────────────────────────────────────────

#[test]
fn a_plain_line_splits_at_the_last_space() {
    assert_eq!(last_word(Posix, "cat /etc/hosts"), ("cat ", "/etc/hosts"));
    assert_eq!(last_word(Posix, "ls"), ("", "ls"));
    assert_eq!(last_word(Posix, ""), ("", ""));
}

#[test]
fn a_trailing_space_starts_a_fresh_word() {
    assert_eq!(last_word(Posix, "cat "), ("cat ", ""));
}

#[test]
fn an_escaped_space_does_not_split_the_word() {
    // A plain whitespace split would hand the completer "Doc" and complete it
    // against the wrong directory.
    assert_eq!(last_word(Posix, "cat My\\ Doc"), ("cat ", "My\\ Doc"));
}

#[test]
fn a_quoted_space_does_not_split_the_word() {
    assert_eq!(last_word(Posix, "cat \"My Doc"), ("cat ", "\"My Doc"));
    assert_eq!(last_word(Posix, "cat 'My Doc"), ("cat ", "'My Doc"));
    assert_eq!(last_word(Posix, "cat 'My Doc' x"), ("cat 'My Doc' ", "x"));
}

#[test]
fn a_backslash_inside_single_quotes_is_literal() {
    // POSIX: nothing is special inside '…', so the quote still closes here.
    assert_eq!(last_word(Posix, "cat 'a\\' b"), ("cat 'a\\' ", "b"));
}

#[test]
fn a_backslash_inside_double_quotes_still_escapes() {
    assert_eq!(last_word(Posix, "cat \"a\\\" b"), ("cat ", "\"a\\\" b"));
}

#[test]
fn splitting_survives_non_ascii_whitespace() {
    // U+3000 is 3 bytes and is `char::is_whitespace`; advancing past it by one
    // byte would split the character and panic.
    assert_eq!(last_word(Posix, "cat\u{3000}doc"), ("cat\u{3000}", "doc"));
}

#[test]
fn a_closed_double_quote_lets_the_following_text_split_normally() {
    // Unlike the earlier double-quote tests, this one actually closes the
    // quote with a plain `"` (not an escaped one), so scanning must return to
    // `Quote::None` and split on the space that follows.
    assert_eq!(
        last_word(Posix, "cat \"My Doc\" x"),
        ("cat \"My Doc\" ", "x")
    );
}

// ─── unescape ─────────────────────────────────────────────────────────────

#[test]
fn unescape_removes_one_layer_of_backslashes_and_quotes() {
    assert_eq!(unescape(Posix, "My\\ Doc"), "My Doc");
    assert_eq!(unescape(Posix, "\"My Doc\""), "My Doc");
    assert_eq!(unescape(Posix, "'My Doc'"), "My Doc");
    assert_eq!(unescape(Posix, "a\\\\b"), "a\\b");
}

#[test]
fn unescape_of_plain_text_is_the_identity() {
    assert_eq!(unescape(Posix, "/usr/local/bin"), "/usr/local/bin");
}

#[test]
fn unescape_resolves_a_backslash_inside_double_quotes() {
    // Double quotes still let a backslash escape the next character, unlike
    // single quotes where nothing is special.
    assert_eq!(unescape(Posix, "\"a\\$b\""), "a$b");
}

#[test]
fn a_half_typed_escape_or_quote_is_not_an_error() {
    // Mid-typing states must degrade, never fail: the completer runs on every
    // keystroke and has nowhere to report a parse error to.
    assert_eq!(unescape(Posix, "abc\\"), "abc");
    assert_eq!(unescape(Posix, "\"abc"), "abc");
    assert_eq!(unescape(Posix, "'abc"), "abc");
}

// ─── escape ───────────────────────────────────────────────────────────────

#[test]
fn escape_protects_whitespace_and_metacharacters() {
    assert_eq!(escape(Posix, "My Doc"), "My\\ Doc");
    assert_eq!(escape(Posix, "a$b"), "a\\$b");
    assert_eq!(escape(Posix, "a*b?"), "a\\*b\\?");
    assert_eq!(escape(Posix, "a;b|c&d"), "a\\;b\\|c\\&d");
}

#[test]
fn escape_leaves_path_separators_alone() {
    // Escaping "/" would defeat the purpose of completing a path.
    assert_eq!(escape(Posix, "/usr/local/bin"), "/usr/local/bin");
}

#[test]
fn tilde_and_hash_are_escaped_only_in_leading_position() {
    // `~/Documents` must stay a home-relative path; a file literally named
    // `~backup` at the start of a word would otherwise be read as a username.
    assert_eq!(escape(Posix, "~backup"), "\\~backup");
    assert_eq!(escape(Posix, "a~b"), "a~b");
    assert_eq!(escape(Posix, "#draft"), "\\#draft");
    assert_eq!(escape(Posix, "note#1"), "note#1");
}

#[test]
fn escape_then_unescape_round_trips() {
    // The load-bearing property: whatever the filesystem holds, splicing the
    // escaped form into a command line makes the shell see the original.
    let corpus = [
        "plain",
        "My Documents",
        "quote'single",
        "quote\"double",
        "back\\slash",
        "dollar$sign",
        "glob*star?mark[bracket]",
        "brace{a,b}",
        "pipe|amp&semi;",
        "redirect<in>out",
        "paren(sub)",
        "bang!hist",
        "~leading-tilde",
        "#leading-hash",
        "tab\there",
        "/abs/path with spaces/file.txt",
        "кириллица и пробел",
        "漢 字",
    ];
    for original in corpus {
        assert_eq!(
            unescape(Posix, &escape(Posix, original)),
            original,
            "round trip failed for {original:?}"
        );
    }
}

#[test]
fn an_escaped_candidate_survives_being_split_out_again() {
    // Splice-then-resplit is what happens when the user Tabs twice: the
    // escaped candidate must come back as one word, not several.
    let line = format!("cat {}", escape(Posix, "My Documents/a b.txt"));
    let (head, word) = last_word(Posix, &line);
    assert_eq!(head, "cat ");
    assert_eq!(unescape(Posix, word), "My Documents/a b.txt");
}
