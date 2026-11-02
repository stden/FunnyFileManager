//! Purpose: Edge-case tests for PowerShell switches/variables/block comments, Typst, and Kotlin.

use super::*;

#[test]
fn powershell_switches_hyphenated_cmdlets_variables_and_block_comments() {
    let mut ps = Highlighter::new(&PathBuf::from("script.ps1"));
    assert_eq!(ps.line("<# start")[0].1, Token::Comment);
    assert_eq!(ps.line("middle")[0].1, Token::Comment);
    let after: Vec<(String, Token)> = ps
        .line("end #> Get-ChildItem -Recurse $env:PATH")
        .into_iter()
        .map(|(t, k)| (t.to_owned(), k))
        .collect();
    assert_eq!(token_of(&after, "end #>"), Token::Comment);
    assert_eq!(token_of(&after, "Get-ChildItem"), Token::Type);
    assert_eq!(token_of(&after, "-Recurse"), Token::Switch);
    assert_eq!(token_of(&after, "$env:PATH"), Token::Type);

    let expr = runs("script.ps1", "Invoke-CustomTool $ - 1");
    assert_eq!(token_of(&expr, "Invoke-CustomTool"), Token::Normal);
    assert_eq!(token_of(&expr, "$"), Token::Punctuation);
    assert_eq!(token_of(&expr, "-"), Token::Punctuation);
    assert_eq!(token_of(&expr, "1"), Token::Number);
}

#[test]
fn typst_meta_lines_do_not_override_open_block_comments() {
    let mut typ = Highlighter::new(&PathBuf::from("doc.typ"));
    assert_eq!(typ.line("/* comment start")[0].1, Token::Comment);
    assert_eq!(typ.line("#set page(width: 10cm)")[0].1, Token::Comment);
    assert_eq!(typ.line("*/")[0].1, Token::Comment);
    assert_eq!(typ.line("#set text(size: 11pt)")[0].1, Token::Meta);

    let code = runs("doc.typ", "let total: length = 12pt // note");
    assert_eq!(token_of(&code, "let"), Token::Keyword);
    assert_eq!(token_of(&code, "length"), Token::Type);
    assert_eq!(token_of(&code, "// note"), Token::Comment);
}

#[test]
fn kotlin_annotations_char_literals_and_trailing_escape_strings() {
    let ann = runs("Main.kt", "    @JvmStatic");
    assert_eq!(ann, vec![("    @JvmStatic".to_owned(), Token::Meta)]);

    let code = runs("Main.kt", r#"val ch: Char = '\n'; val raw = "trail\"#);
    assert_eq!(token_of(&code, "val"), Token::Keyword);
    assert_eq!(token_of(&code, "Char"), Token::Type);
    assert_eq!(token_of(&code, r"'\n'"), Token::Str);
    assert_eq!(token_of(&code, r#""trail\"#), Token::Str);
}
