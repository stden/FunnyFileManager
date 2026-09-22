//! End-to-end tests: syntax highlighting as it reaches the screen.
//!
//! `src/highlight/tests.rs` proves the tokenizer classifies text correctly.
//! That is a different claim from "the editor draws it in the right colour",
//! and the gap between them is where the real bugs live: a token class that no
//! style maps to, a horizontal scroll that slices a span at the wrong byte, a
//! highlighter seeded at the visible line instead of at the top of the file.
//!
//! So these tests open a real file in the real editor and read the colour of
//! individual cells back out of the frame buffer.

use crossterm::event::{Event, KeyCode, KeyEvent, KeyEventKind, KeyEventState, KeyModifiers};
use funny_file_manager::highlight::{Highlighter, Token};
use funny_file_manager::models::{App, Panel};
use funny_file_manager::{run_app_with_event_reader, WritableTestBackend};
use ratatui::style::Color;
use ratatui::Terminal;
use std::fs;
use std::io;
use std::path::Path;
use tempfile::TempDir;

const SCREEN_W: u16 = 90;
const SCREEN_H: u16 = 20;

// The editor's palette, as `src/ui/palette.rs` defines it. Repeated here on
// purpose: a test that imported the constants would still pass if every colour
// were changed at once, which is exactly the change that should fail it.
const CODE_NORMAL: Color = Color::Rgb(120, 200, 230);
const CODE_KEYWORD: Color = Color::Rgb(255, 255, 255);
const CODE_STRING: Color = Color::Rgb(235, 185, 70);
const CODE_NUMBER: Color = Color::Rgb(120, 220, 120);
const CODE_COMMENT: Color = Color::Rgb(215, 135, 65);
const CODE_TYPE: Color = Color::Rgb(140, 220, 200);
const CODE_DIRECTIVE: Color = Color::Rgb(150, 220, 120);

/// One line of the file, opened in the editor, as (character, colour) pairs.
///
/// Row 1 of the screen: the editor spends row 0 on its bar and every row after
/// that on the file.
fn painted(name: &str, content: &str, row: u16) -> Vec<(char, Color)> {
    painted_with_key(name, content, row, KeyCode::F(4))
}

fn painted_with_key(name: &str, content: &str, row: u16, open: KeyCode) -> Vec<(char, Color)> {
    let dir = TempDir::new().expect("dir");
    let other = TempDir::new().expect("dir");
    fs::write(dir.path().join(name), content).expect("seed");

    let mut app = App::new().expect("app");
    app.left_panel = Panel::new(dir.path().to_path_buf()).expect("panel");
    app.right_panel = Panel::new(other.path().to_path_buf()).expect("panel");

    let key = |code| {
        Event::Key(KeyEvent {
            code,
            modifiers: KeyModifiers::empty(),
            kind: KeyEventKind::Press,
            state: KeyEventState::empty(),
        })
    };
    let mut events = vec![key(KeyCode::Down), key(open)].into_iter();

    let mut terminal =
        Terminal::new(WritableTestBackend::new(SCREEN_W, SCREEN_H)).expect("terminal");
    // The stream runs out with the editor still open, which is how the test
    // says "stop here" — the frame is what it wants, not a clean exit.
    let _ = run_app_with_event_reader(&mut terminal, app, || {
        events
            .next()
            .ok_or_else(|| io::Error::new(io::ErrorKind::UnexpectedEof, "stop"))
    });

    let buffer = terminal.backend().buffer();
    (0..SCREEN_W)
        .map(|x| {
            let cell = &buffer[(x, row)];
            (cell.symbol().chars().next().unwrap_or(' '), cell.fg)
        })
        .collect()
}

/// The colour the run starting at `needle` is painted in.
fn colour_of(row: &[(char, Color)], needle: &str) -> Color {
    let text: String = row.iter().map(|(ch, _)| *ch).collect();
    let at = text
        .find(needle)
        .unwrap_or_else(|| panic!("{needle:?} is not on the row {text:?}"));
    row[at].1
}

#[test]
fn rust_keywords_types_strings_and_comments_reach_the_screen_in_colour() {
    let row = painted("main.rs", "let name = \"hi\"; // greet\n", 1);

    assert_eq!(colour_of(&row, "let"), CODE_KEYWORD, "keyword");
    assert_eq!(colour_of(&row, "name"), CODE_NORMAL, "identifier");
    assert_eq!(colour_of(&row, "\"hi\""), CODE_STRING, "string");
    assert_eq!(colour_of(&row, "// greet"), CODE_COMMENT, "comment");
}

#[test]
fn a_number_is_painted_apart_from_the_identifier_beside_it() {
    let row = painted("main.rs", "let x = 3.14;\n", 1);
    assert_eq!(colour_of(&row, "3.14"), CODE_NUMBER);
    assert_eq!(colour_of(&row, "x"), CODE_NORMAL);
}

/// The cross-line case, and the reason the highlighter is seeded at line 0
/// rather than at the first visible line: a block comment opened above still
/// colours what is inside it.
#[test]
fn a_block_comment_stays_coloured_across_lines() {
    let content = "/* opening\nstill inside\ndone */ let x = 1;\n";
    assert_eq!(
        colour_of(&painted("main.rs", content, 2), "still"),
        CODE_COMMENT,
        "the second line is inside the comment"
    );
    let closing = painted("main.rs", content, 3);
    assert_eq!(colour_of(&closing, "done"), CODE_COMMENT, "up to the close");
    assert_eq!(
        colour_of(&closing, "let"),
        CODE_KEYWORD,
        "and code after it is code again"
    );
}

#[test]
fn python_is_highlighted_by_its_own_rules() {
    let row = painted("app.py", "def greet(name: str) -> str:\n", 1);
    assert_eq!(colour_of(&row, "def"), CODE_KEYWORD);
    // A type is its own colour, not an identifier's.
    assert_eq!(colour_of(&row, "str"), CODE_TYPE);
    // A decorator is a directive, which is code — not a comment.
    assert_eq!(
        colour_of(&painted("app.py", "@property\n", 1), "@property"),
        CODE_DIRECTIVE
    );
}

/// `#` opens a directive in C and a comment in shell. Painting `#include` as a
/// comment is the visible failure this pins.
#[test]
fn a_leading_hash_is_read_differently_by_c_and_by_shell() {
    assert_eq!(
        colour_of(&painted("main.c", "#include <stdio.h>\n", 1), "#include"),
        CODE_DIRECTIVE,
        "a directive is code, not a comment"
    );
    assert_eq!(
        colour_of(&painted("run.sh", "# a note\n", 1), "# a note"),
        CODE_COMMENT
    );
}

/// Batch files had no entry in the syntax table at all and rendered plain.
#[test]
fn batch_files_are_highlighted() {
    let row = painted("build.cmd", "exit /b 1\n", 1);
    assert_eq!(colour_of(&row, "exit"), CODE_KEYWORD);
    // Far paints a switch the same green it gives a number.
    assert_eq!(colour_of(&row, "/b"), CODE_NUMBER, "switch");
    assert_eq!(colour_of(&row, "1"), CODE_NUMBER, "number");
}

#[test]
fn markdown_headings_are_set_apart_from_body_text() {
    let content = "# Title\n\nbody text\n";
    assert_eq!(
        colour_of(&painted("README.md", content, 1), "# Title"),
        CODE_KEYWORD
    );
    assert_eq!(
        colour_of(&painted("README.md", content, 3), "body"),
        CODE_NORMAL
    );
}

/// A file the table does not know must render, and render plainly — an unknown
/// extension is not a reason to lose the text.
#[test]
fn an_unknown_extension_renders_plainly_but_completely() {
    let row = painted("notes.xyz", "let name = \"hi\";\n", 1);
    let text: String = row.iter().map(|(ch, _)| *ch).collect();
    assert!(text.starts_with("let name = \"hi\";"), "got {text:?}");
    assert_eq!(colour_of(&row, "let"), CODE_NORMAL, "nothing is a keyword");
    assert_eq!(
        colour_of(&row, "\"hi\""),
        CODE_NORMAL,
        "nothing is a string"
    );
}

/// Highlighting must not eat characters: every cell of the line has to be the
/// character that was in the file.
#[test]
fn the_painted_line_is_the_line_that_was_in_the_file() {
    let source = "let mut счёт = \"значение\"; // комментарий";
    let row = painted("main.rs", &format!("{source}\n"), 1);
    let text: String = row.iter().map(|(ch, _)| *ch).collect();
    assert!(
        text.starts_with(source),
        "the multibyte line must survive being coloured:\n{text:?}"
    );
    assert_eq!(colour_of(&row, "\"значение\""), CODE_STRING);
    assert_eq!(colour_of(&row, "// комментарий"), CODE_COMMENT);
}

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
