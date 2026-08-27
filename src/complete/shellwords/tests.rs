//! Purpose: Pins the quoting rules completion and the shell must agree on.

use super::*;

// ─── last_word ────────────────────────────────────────────────────────────

#[test]
fn a_plain_line_splits_at_the_last_space() {
    assert_eq!(last_word("cat /etc/hosts"), ("cat ", "/etc/hosts"));
    assert_eq!(last_word("ls"), ("", "ls"));
    assert_eq!(last_word(""), ("", ""));
}

#[test]
fn a_trailing_space_starts_a_fresh_word() {
    assert_eq!(last_word("cat "), ("cat ", ""));
}

#[test]
fn an_escaped_space_does_not_split_the_word() {
    // A plain whitespace split would hand the completer "Doc" and complete it
    // against the wrong directory.
    assert_eq!(last_word("cat My\\ Doc"), ("cat ", "My\\ Doc"));
}

#[test]
fn a_quoted_space_does_not_split_the_word() {
    assert_eq!(last_word("cat \"My Doc"), ("cat ", "\"My Doc"));
    assert_eq!(last_word("cat 'My Doc"), ("cat ", "'My Doc"));
    assert_eq!(last_word("cat 'My Doc' x"), ("cat 'My Doc' ", "x"));
}

#[test]
fn a_backslash_inside_single_quotes_is_literal() {
    // POSIX: nothing is special inside '…', so the quote still closes here.
    assert_eq!(last_word("cat 'a\\' b"), ("cat 'a\\' ", "b"));
}

#[test]
fn a_backslash_inside_double_quotes_still_escapes() {
    assert_eq!(last_word("cat \"a\\\" b"), ("cat ", "\"a\\\" b"));
}

#[test]
fn splitting_survives_non_ascii_whitespace() {
    // U+3000 is 3 bytes and is `char::is_whitespace`; advancing past it by one
    // byte would split the character and panic.
    assert_eq!(last_word("cat\u{3000}doc"), ("cat\u{3000}", "doc"));
}

// ─── unescape ─────────────────────────────────────────────────────────────

#[test]
fn unescape_removes_one_layer_of_backslashes_and_quotes() {
    assert_eq!(unescape("My\\ Doc"), "My Doc");
    assert_eq!(unescape("\"My Doc\""), "My Doc");
    assert_eq!(unescape("'My Doc'"), "My Doc");
    assert_eq!(unescape("a\\\\b"), "a\\b");
}

#[test]
fn unescape_of_plain_text_is_the_identity() {
    assert_eq!(unescape("/usr/local/bin"), "/usr/local/bin");
}

#[test]
fn a_half_typed_escape_or_quote_is_not_an_error() {
    // Mid-typing states must degrade, never fail: the completer runs on every
    // keystroke and has nowhere to report a parse error to.
    assert_eq!(unescape("abc\\"), "abc");
    assert_eq!(unescape("\"abc"), "abc");
    assert_eq!(unescape("'abc"), "abc");
}

// ─── escape ───────────────────────────────────────────────────────────────

#[test]
fn escape_protects_whitespace_and_metacharacters() {
    assert_eq!(escape("My Doc"), "My\\ Doc");
    assert_eq!(escape("a$b"), "a\\$b");
    assert_eq!(escape("a*b?"), "a\\*b\\?");
    assert_eq!(escape("a;b|c&d"), "a\\;b\\|c\\&d");
}

#[test]
fn escape_leaves_path_separators_alone() {
    // Escaping "/" would defeat the purpose of completing a path.
    assert_eq!(escape("/usr/local/bin"), "/usr/local/bin");
}

#[test]
fn tilde_and_hash_are_escaped_only_in_leading_position() {
    // `~/Documents` must stay a home-relative path; a file literally named
    // `~backup` at the start of a word would otherwise be read as a username.
    assert_eq!(escape("~backup"), "\\~backup");
    assert_eq!(escape("a~b"), "a~b");
    assert_eq!(escape("#draft"), "\\#draft");
    assert_eq!(escape("note#1"), "note#1");
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
            unescape(&escape(original)),
            original,
            "round trip failed for {original:?}"
        );
    }
}

#[test]
fn an_escaped_candidate_survives_being_split_out_again() {
    // Splice-then-resplit is what happens when the user Tabs twice: the
    // escaped candidate must come back as one word, not several.
    let line = format!("cat {}", escape("My Documents/a b.txt"));
    let (head, word) = last_word(&line);
    assert_eq!(head, "cat ");
    assert_eq!(unescape(word), "My Documents/a b.txt");
}
