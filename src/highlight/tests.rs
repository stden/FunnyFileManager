//! Unit tests: token classification, and the one thing that crosses lines.

use super::*;
use std::path::{Path, PathBuf};

#[cfg(feature = "colorer")]
mod colorer_paint;
mod file_names;
mod languages;
mod markdown;
mod markup;

/// Concatenating the runs must reproduce the line exactly.
///
/// The tokenizer walks byte offsets over UTF-8, and every off-by-one there is
/// either a panic on a char boundary or a silently dropped character. This
/// invariant catches both, so every test below asserts it.
fn runs(path: &str, line: &str) -> Vec<(String, Token)> {
    let mut highlighter = Highlighter::new(&PathBuf::from(path));
    let runs = highlighter.line(line);
    let rejoined: String = runs.iter().map(|(text, _)| *text).collect();
    assert_eq!(rejoined, line, "runs must cover the line exactly");
    runs.into_iter()
        .map(|(text, token)| (text.to_owned(), token))
        .collect()
}

/// The token a given substring was classified as.
fn token_of(runs: &[(String, Token)], needle: &str) -> Token {
    runs.iter()
        .find(|(text, _)| text == needle)
        .unwrap_or_else(|| panic!("no run {needle:?} in {runs:?}"))
        .1
}

#[test]
fn rust_keywords_types_strings_and_numbers_are_separated() {
    let line = r#"    let name: String = "hi".to_string(); // greet"#;
    let out = runs("a.rs", line);

    assert_eq!(token_of(&out, "let"), Token::Keyword);
    assert_eq!(token_of(&out, "String"), Token::Type);
    assert_eq!(token_of(&out, r#""hi""#), Token::Str);
    assert_eq!(token_of(&out, "// greet"), Token::Comment);
    assert_eq!(token_of(&out, "name"), Token::Normal);
}

#[test]
fn every_syntax_table_keeps_keywords_and_types_sorted_for_binary_search() {
    for name in [
        "a.rs",
        "a.py",
        "a.kt",
        "a.cpp",
        "a.sh",
        "a.cmd",
        "a.ps1",
        "a.toml",
        "a.env",
        "a.json",
        "a.xml",
        "a.md",
        "a.typ",
        "Makefile",
        "Cargo.lock",
    ] {
        let syntax = syntax_for(Path::new(name)).expect(name);
        assert!(syntax.keywords.is_sorted(), "{name} keywords unsorted");
        assert!(syntax.types.is_sorted(), "{name} types unsorted");
    }
}

#[test]
fn a_number_is_one_run_including_its_suffix_and_point() {
    let out = runs("a.rs", "let x = 3.14f64;");
    assert_eq!(token_of(&out, "3.14f64"), Token::Number);
}

/// An escaped quote does not end the string, and an unterminated one runs to
/// the end of the line — which is what a half-typed string looks like.
#[test]
fn strings_handle_escapes_and_missing_terminators() {
    let out = runs("a.rs", r#"let s = "a\"b";"#);
    assert_eq!(token_of(&out, r#""a\"b""#), Token::Str);

    let out = runs("a.rs", r#"let s = "unfinished"#);
    assert_eq!(token_of(&out, r#""unfinished"#), Token::Str);
}

/// The one piece of cross-line state. A highlighter seeded per line colours
/// only the first line of a licence header and leaves the rest looking like
/// code.
#[test]
fn a_block_comment_keeps_colouring_until_it_closes() {
    let mut highlighter = Highlighter::new(&PathBuf::from("a.rs"));

    assert_eq!(highlighter.line("/* opening")[0].1, Token::Comment);
    assert_eq!(highlighter.line("still inside")[0].1, Token::Comment);

    let closing = highlighter.line("done */ let x = 1;");
    assert_eq!(closing[0].1, Token::Comment);
    assert_eq!(closing[0].0, "done */");
    // Code after the close is code again.
    let after: Vec<(String, Token)> = closing
        .into_iter()
        .map(|(t, k)| (t.to_owned(), k))
        .collect();
    assert_eq!(token_of(&after, "let"), Token::Keyword);
}

#[test]
fn a_block_comment_opened_and_closed_on_one_line_does_not_leak() {
    let mut highlighter = Highlighter::new(&PathBuf::from("a.rs"));
    let out = highlighter.line("let /* mid */ x = 1;");
    assert_eq!(
        out.iter().find(|(t, _)| *t == "/* mid */").unwrap().1,
        Token::Comment
    );

    // The next line must not still be inside a comment.
    assert_eq!(highlighter.line("let y = 2;")[0].1, Token::Keyword);
}

/// Non-ASCII must not split a byte offset mid-character.
#[test]
fn multibyte_text_is_never_cut_mid_character() {
    let out = runs("a.rs", r#"let имя = "значение"; // комментарий"#);
    assert_eq!(token_of(&out, "let"), Token::Keyword);
    assert_eq!(token_of(&out, r#""значение""#), Token::Str);
    assert_eq!(token_of(&out, "// комментарий"), Token::Comment);

    // And in Markdown, whose bullet arm slices by byte length too.
    runs("a.md", "- пункт списка");
    runs("a.md", "# Заголовок");
}

/// An empty line must produce no runs and must not panic.
#[test]
fn an_empty_line_is_handled() {
    let mut highlighter = Highlighter::new(&PathBuf::from("a.rs"));
    assert_eq!(highlighter.line("").len(), 0);
    #[cfg(feature = "colorer")]
    {
        assert_eq!(regions::utf16_to_char_idx("a🦀b", 0), 0);
        assert_eq!(regions::utf16_to_char_idx("a🦀b", 3), 2);
        assert_eq!(regions::utf16_to_char_idx("a🦀b", 99), 3);
    }
}
