//! Purpose: Rust, Python, C, shell, batch and Markdown colours, and unknown extensions.

use super::*;

// The editor's palette, as `src/ui/palette.rs` defines it. Repeated here on
// purpose: a test that imported the constants would still pass if every colour
// were changed at once, which is exactly the change that should fail it.
const CODE_NORMAL: Color = Color::Rgb(0, 255, 255);

const CODE_NUMBER: Color = Color::Rgb(255, 0, 255);

const CODE_DIRECTIVE: Color = Color::Rgb(0, 255, 0);

/// One line of the file, opened in the editor, as (character, colour) pairs.
///
/// Row 1 of the screen: the editor spends row 0 on its bar and every row after
/// that on the file.
fn painted(name: &str, content: &str, row: u16) -> Vec<(char, Color)> {
    painted_with_key(name, content, row, KeyCode::F(4))
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
    let doc = "\"\"\"Shared runtime.\nStdlib and tools.\n\"\"\"\ndef ok():\n";
    assert_eq!(
        colour_of(&painted("app.py", doc, 1), "Shared"),
        CODE_COMMENT
    );
    assert_eq!(colour_of(&painted("app.py", doc, 2), "and"), CODE_COMMENT);
    assert_eq!(
        colour_of(&painted("app.py", doc, 3), "\"\"\""),
        CODE_COMMENT
    );
    assert_eq!(colour_of(&painted("app.py", doc, 4), "def"), CODE_KEYWORD);
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
