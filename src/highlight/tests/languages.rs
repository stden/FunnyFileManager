//! Purpose: per-language token rules: C/shell `#`, Python decorators, batch switches, config section headers, shell and PowerShell variables.

use super::*;

/// `#` opens a preprocessor directive in C and a comment in shell. Getting
/// this backwards paints every `#include` as a comment.
#[test]
fn a_leading_hash_is_a_directive_in_c_and_a_comment_in_shell() {
    assert_eq!(runs("a.c", "#include <stdio.h>")[0].1, Token::Meta);
    assert_eq!(runs("a.sh", "# just a note")[0].1, Token::Comment);
}

#[test]
fn python_decorators_and_keywords_are_recognised() {
    assert_eq!(runs("a.py", "@property")[0].1, Token::Meta);
    let out = runs("a.py", "def greet(name: str) -> str:");
    assert_eq!(token_of(&out, "def"), Token::Keyword);
    assert_eq!(token_of(&out, "str"), Token::Type);
}

/// Windows batch. Far paints it sparely — the switches and numbers are what
/// stand out, and the `.cmd`/`.bat` extensions had no entry at all before.
#[test]
fn batch_files_highlight_switches_numbers_and_comments() {
    let out = runs("fast-build.cmd", "exit /b 1");
    assert_eq!(token_of(&out, "exit"), Token::Keyword);
    assert_eq!(token_of(&out, "/b"), Token::Switch);
    assert_eq!(token_of(&out, "1"), Token::Number);

    assert_eq!(runs("x.bat", ":: a note")[0].1, Token::Comment);
    assert_eq!(runs("x.cmd", "rem a note")[0].1, Token::Comment);

    // `set "NAME=value"` groups with quotes rather than quoting a literal, so
    // the quotes must not paint half the file as one string.
    let assign = runs("x.cmd", r#"set "SCRIPT_DIR=%~dp0""#);
    assert!(
        assign.iter().all(|(_, token)| *token != Token::Str),
        "batch quotes are grouping, not strings: {assign:?}"
    );
}

/// A path is not a switch: `/` only opens one when a letter follows it.
#[test]
fn a_slash_in_a_path_is_not_a_switch() {
    let out = runs("x.cmd", "echo C:/tmp/ and / alone");
    assert!(
        out.iter()
            .all(|(text, token)| *token != Token::Switch || text.starts_with('/')),
        "{out:?}"
    );
    // The bare slash with a space after it stays punctuation.
    assert_eq!(token_of(&out, "/"), Token::Punctuation);
}

/// A section header is the line's shape, and must not be mistaken for one when
/// the language has no sections.
#[test]
fn section_headers_are_only_a_thing_in_config_files() {
    assert_eq!(runs("app.ini", "  [server]  ")[0].1, Token::Keyword);
    assert_eq!(runs("a.toml", "[dependencies]")[0].1, Token::Keyword);
    // Rust has `[` too. A section header swallows the whole line as one run,
    // so the test is that this line is still tokenized rather than swallowed.
    assert!(runs("a.rs", "[1, 2]").len() > 1, "not a section header");
}

/// `$name` is a variable, checked before the digit and word rules so it is not
/// misread as a number (`$1`) or a bare word (`$name`).
#[test]
fn shell_variables_are_their_own_token() {
    let out = runs("a.sh", "echo $HOME $1 $user_name");
    assert_eq!(token_of(&out, "$HOME"), Token::Type);
    assert_eq!(token_of(&out, "$1"), Token::Type);
    assert_eq!(token_of(&out, "$user_name"), Token::Type);
}

/// A PowerShell variable can carry a scope qualifier, like `$env:PATH` — the
/// colon is part of the variable's name, not punctuation after it.
#[test]
fn powershell_variables_include_a_scope_qualifier() {
    let out = runs("a.ps1", "Write-Host $env:PATH");
    assert_eq!(token_of(&out, "Write-Host"), Token::Type);
    assert_eq!(token_of(&out, "$env:PATH"), Token::Type);
}
