//! Purpose: PowerShell, shell variables and .env files in the editor and the viewer.

use super::*;

// ─── PowerShell ─────────────────────────────────────────────────────────────

/// `.ps1` used to be highlighted as a Windows batch file, which shares almost
/// nothing with it: no block comments, no real strings, and none of the
/// `$variables` and verb-noun cmdlets a PowerShell script is mostly made of.
#[test]
fn powershell_is_not_treated_as_a_batch_file() {
    let lines = vec![
        "$count = 3".to_owned(),
        "New-Object System.Diagnostics.Process".to_owned(),
    ];
    let mut highlighter = Highlighter::for_file(Path::new("script.ps1"), &lines);

    let tokens = highlighter.line(&lines[0]);
    assert!(
        tokens.iter().any(|(text, _)| *text == "$count"),
        "a variable is one token, prefix included: {tokens:?}"
    );

    let tokens = highlighter.line(&lines[1]);
    assert!(
        tokens.iter().any(|(text, _)| *text == "New-Object"),
        "a cmdlet is one token, not a word and a switch: {tokens:?}"
    );
}

/// `<# ... #>` spans lines, so the highlighter has to carry the fact across
/// them — the one piece of state it keeps.
#[test]
fn a_powershell_block_comment_covers_the_lines_between_its_ends() {
    let lines = vec![
        "<#".to_owned(),
        ".SYNOPSIS".to_owned(),
        "  what it does".to_owned(),
        "#>".to_owned(),
        "$after = 1".to_owned(),
    ];
    let mut highlighter = Highlighter::for_file(Path::new("doc.ps1"), &lines);

    for line in &lines[..4] {
        for (text, token) in highlighter.line(line) {
            assert_eq!(token, Token::Comment, "{text:?} is inside the block");
        }
    }
    let after = highlighter.line(&lines[4]);
    assert!(
        after.iter().all(|(_, token)| *token != Token::Comment),
        "and the block ends: {after:?}"
    );
}

/// Parameters and the word-shaped operators look the same and are the same
/// kind of thing, so they get the same colour.
#[test]
fn powershell_parameters_and_operators_stand_out() {
    let lines = vec!["if ($a -lt 10) { Get-Item -Path $p }".to_owned()];
    let mut highlighter = Highlighter::for_file(Path::new("s.ps1"), &lines);

    let tokens = highlighter.line(&lines[0]);
    for expected in ["-lt", "-Path"] {
        assert!(
            tokens
                .iter()
                .any(|(text, token)| *text == expected && *token == Token::Switch),
            "{expected} must be a switch: {tokens:?}"
        );
    }
    assert!(tokens
        .iter()
        .any(|(text, token)| *text == "if" && *token == Token::Keyword));
}

/// Shell scripts gained the same variable rule, since `$HOME` is as much the
/// subject of a shell line as `$env` is of a PowerShell one.
#[test]
fn shell_variables_are_one_token_too() {
    let lines = vec!["echo \"$HOME/bin\"".to_owned(), "cd $PWD".to_owned()];
    let mut highlighter = Highlighter::for_file(Path::new("go.sh"), &lines);

    highlighter.line(&lines[0]);
    let tokens = highlighter.line(&lines[1]);
    assert!(tokens.iter().any(|(text, _)| *text == "$PWD"), "{tokens:?}");
}

#[test]
fn dotenv_highlighting_reaches_both_editor_and_viewer() {
    for open in [KeyCode::F(3), KeyCode::F(4)] {
        for name in [".env", ".env.production.local"] {
            let source = "export URL=\"${HOST}/path#fragment\" # local";
            let row = painted_with_key(name, source, 1, open);
            assert_eq!(colour_of(&row, "export"), CODE_KEYWORD);
            assert_eq!(colour_of(&row, "URL"), CODE_TYPE);
            assert_eq!(colour_of(&row, "${HOST}"), CODE_TYPE);
            assert_eq!(colour_of(&row, "/path#fragment"), CODE_STRING);
            assert_eq!(colour_of(&row, "# local"), CODE_COMMENT);
            let text: String = row.iter().map(|(ch, _)| *ch).collect();
            assert!(text.starts_with(source), "{name}, {open:?}: {text:?}");
        }
    }
}
