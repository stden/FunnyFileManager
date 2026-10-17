//! Purpose: Pins the quoting rules completion and cmd must agree on — no backslash escape, double quotes only.

use super::*;
use crate::complete::PathSyntax::{Posix, Windows};

// ─── last_word ────────────────────────────────────────────────────────────

#[test]
fn a_backslash_never_splits_or_joins_a_word_under_windows() {
    // `dir C:\work\fi` names a path; under the Posix rules `\x` and `\f` were
    // escapes, which happened to leave the word whole but not its meaning.
    assert_eq!(
        last_word(Windows, "dir C:\\work\\fi"),
        ("dir ", "C:\\work\\fi")
    );
    // Only quotes join a word: `\ ` protects nothing in cmd.
    assert_eq!(last_word(Windows, "dir C:\\ x"), ("dir C:\\ ", "x"));
    assert_eq!(last_word(Posix, "dir C:\\ x"), ("dir ", "C:\\ x"));
}

#[test]
fn double_quotes_join_a_word_and_single_quotes_do_not_under_windows() {
    assert_eq!(
        last_word(Windows, "type \"My Docs\\re"),
        ("type ", "\"My Docs\\re")
    );
    assert_eq!(
        last_word(Windows, "type \"My Docs\" x"),
        ("type \"My Docs\" ", "x")
    );
    // cmd knows no single quote.
    assert_eq!(last_word(Windows, "type 'My Docs"), ("type 'My ", "Docs"));
}

// ─── unescape ─────────────────────────────────────────────────────────────

#[test]
fn unescape_keeps_every_backslash_under_windows() {
    // The bug: `C:\work\fi` unescaped to `C:workfi` and matched nothing.
    assert_eq!(unescape(Windows, "C:\\work\\fi"), "C:\\work\\fi");
    assert_eq!(unescape(Posix, "C:\\work\\fi"), "C:workfi");
}

#[test]
fn unescape_drops_double_quotes_only_under_windows() {
    assert_eq!(unescape(Windows, "\"My Docs\"\\rep"), "My Docs\\rep");
    assert_eq!(unescape(Windows, "\"My Docs"), "My Docs");
    assert_eq!(unescape(Windows, "it's"), "it's");
}

// ─── escape ───────────────────────────────────────────────────────────────

#[test]
fn escape_double_quotes_a_word_cmd_would_split_or_interpret() {
    assert_eq!(escape(Windows, "My Docs"), "\"My Docs\"");
    assert_eq!(escape(Windows, "a&b"), "\"a&b\"");
    assert_eq!(escape(Windows, "a|b<c>d^e"), "\"a|b<c>d^e\"");
    assert_eq!(escape(Windows, "(x)"), "\"(x)\"");
    assert_eq!(escape(Windows, "a;b,c=d"), "\"a;b,c=d\"");
}

#[test]
fn escape_leaves_a_plain_windows_path_alone() {
    // No backslash escaping: the backslashes are the path.
    assert_eq!(escape(Windows, "C:\\work\\file.txt"), "C:\\work\\file.txt");
    // cmd expands no tilde and opens no comment.
    assert_eq!(escape(Windows, "~backup"), "~backup");
    assert_eq!(escape(Windows, "#draft"), "#draft");
    assert_eq!(escape(Windows, "it's"), "it's");
}

#[test]
fn escape_keeps_trailing_separators_outside_the_quotes() {
    // `"My Docs\"` reads as `My Docs"` to a program cmd starts: the C runtime
    // takes `\"` as a literal quote. `"My Docs"\` reads as `My Docs\` to cmd
    // and to that program alike, and stays a prefix as the user types on.
    assert_eq!(escape(Windows, "My Docs\\"), "\"My Docs\"\\");
    assert_eq!(escape(Windows, "My Docs/"), "\"My Docs\"/");
    assert_eq!(
        escape(Windows, "C:\\My Docs\\sub\\"),
        "\"C:\\My Docs\\sub\"\\"
    );
}

#[test]
fn escape_then_unescape_round_trips_under_windows() {
    // A Windows filename cannot hold a `"`, so nothing else needs an escape.
    let corpus = [
        "plain",
        "My Documents",
        "My Documents\\",
        "C:\\work\\My Docs\\a b.txt",
        "a&b|c<d>e^f",
        "paren(sub)",
        "semi;comma,equals=",
        "quote'single",
        "~leading-tilde",
        "#leading-hash",
        "tab\there",
        "кириллица и пробел\\",
    ];
    for original in corpus {
        assert_eq!(
            unescape(Windows, &escape(Windows, original)),
            original,
            "round trip failed for {original:?}"
        );
    }
}

#[test]
fn a_quoted_candidate_survives_being_split_out_again_under_windows() {
    let line = format!("type {}", escape(Windows, "My Docs\\a b.txt"));
    let (head, word) = last_word(Windows, &line);
    assert_eq!(head, "type ");
    assert_eq!(unescape(Windows, word), "My Docs\\a b.txt");
    // Typing on into a completed directory keeps it one word.
    let line = format!("type {}rep", escape(Windows, "My Docs\\"));
    assert_eq!(last_word(Windows, &line).1, "\"My Docs\"\\rep");
}
