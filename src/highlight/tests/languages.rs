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

#[test]
fn python_triple_quoted_docstrings_stay_comments_across_lines() {
    let mut highlighter = Highlighter::new(&PathBuf::from("agentkit.py"));
    let first = highlighter.line("\"\"\"Shared runtime for skill scripts.");
    assert_eq!(
        first,
        vec![("\"\"\"Shared runtime for skill scripts.", Token::Comment)]
    );

    let middle = highlighter.line("Stdlib only and user's path with 0 errors.");
    assert_eq!(
        middle,
        vec![("Stdlib only and user's path with 0 errors.", Token::Comment)]
    );

    let decorator_like = highlighter.line("@not_a_decorator_inside_docstring");
    assert_eq!(
        decorator_like,
        vec![("@not_a_decorator_inside_docstring", Token::Comment)]
    );

    let closing = highlighter.line("\"\"\"");
    assert_eq!(closing, vec![("\"\"\"", Token::Comment)]);

    let after = highlighter.line("from pathlib import Path");
    assert_eq!(after[0], ("from", Token::Keyword));

    let single_quote_doc = highlighter.line("'''single line docstring'''");
    assert_eq!(
        single_quote_doc,
        vec![("'''single line docstring'''", Token::Comment)]
    );
    assert_eq!(highlighter.line("def run():")[0], ("def", Token::Keyword));
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

/// Kotlin has its own words: `fun`, `val` and `when` were plain text under
/// the C table it used to share.
#[test]
fn kotlin_keywords_types_annotations_and_strings_are_recognised() {
    let out = runs("Main.kt", r#"    val name: String = "hi" // greet"#);
    assert_eq!(token_of(&out, "val"), Token::Keyword);
    assert_eq!(token_of(&out, "String"), Token::Type);
    assert_eq!(token_of(&out, r#""hi""#), Token::Str);
    assert_eq!(token_of(&out, "// greet"), Token::Comment);

    let out = runs(
        "Main.kt",
        "suspend fun load(id: Int): Unit = when (id) { else -> Unit }",
    );
    for keyword in ["suspend", "fun", "when", "else"] {
        assert_eq!(token_of(&out, keyword), Token::Keyword, "{keyword}");
    }
    assert_eq!(token_of(&out, "Int"), Token::Type);
    assert_eq!(runs("Main.kt", "@JvmStatic")[0].1, Token::Meta);
}

/// Gradle's Kotlin build scripts are Kotlin.
#[test]
fn a_kotlin_script_is_kotlin() {
    let out = runs("build.gradle.kts", r#"val kotlinVersion = "2.1.0""#);
    assert_eq!(token_of(&out, "val"), Token::Keyword);
}

/// Typst: `#`-prefixed lines are directives; keywords, types and strings work
/// in code lines; block and line comments survive.
#[test]
fn typst_hash_directives_keywords_types_and_strings_are_recognised() {
    // A line that starts with `#` is a code injection — coloured as a whole.
    assert_eq!(runs("main.typ", "#let x = 1")[0].1, Token::Meta);
    assert_eq!(runs("doc.typ", "#set text(size: 12pt)")[0].1, Token::Meta);

    // Keywords and types in plain code-mode lines (e.g. inside `{ }` blocks).
    let out = runs("main.typ", r#"let name: str = "Alice" // greeting"#);
    assert_eq!(token_of(&out, "let"), Token::Keyword);
    assert_eq!(token_of(&out, "str"), Token::Type);
    assert_eq!(token_of(&out, r#""Alice""#), Token::Str);
    assert_eq!(token_of(&out, "// greeting"), Token::Comment);

    // Boolean and `none` literals are keywords; `and` is a keyword operator.
    let out = runs("main.typ", "let ok = true and none");
    assert_eq!(token_of(&out, "true"), Token::Keyword);
    assert_eq!(token_of(&out, "and"), Token::Keyword);
    assert_eq!(token_of(&out, "none"), Token::Keyword);
}
