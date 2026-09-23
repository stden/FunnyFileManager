//! Purpose: Markdown line-shape classification: headings, fences, bullets and plain paragraphs.

use super::*;

/// Markdown is line-shaped, so it is classified by line rather than tokenized.
#[test]
fn markdown_uses_line_shape_and_tracks_fences() {
    let mut highlighter = Highlighter::new(&PathBuf::from("README.md"));
    assert_eq!(highlighter.line("# Title")[0].1, Token::Keyword);
    assert_eq!(highlighter.line("> quoted")[0].1, Token::Comment);

    assert_eq!(highlighter.line("```rust")[0].1, Token::Comment);
    // Inside the fence, body text is not headings.
    assert_eq!(highlighter.line("# not a heading")[0].1, Token::Str);
    assert_eq!(highlighter.line("```")[0].1, Token::Comment);
    assert_eq!(highlighter.line("# heading again")[0].1, Token::Keyword);
}

#[test]
fn a_markdown_bullet_is_marked_without_swallowing_the_text() {
    let out = runs("a.md", "- an item");
    assert_eq!(out[0].1, Token::Meta);
    assert_eq!(out[1].0, " an item");
}

/// Prose that is not a heading, quote, fence, or bullet is the common case for
/// a Markdown file and must fall through every shape check unchanged.
#[test]
fn a_markdown_paragraph_with_no_special_shape_is_left_normal() {
    let out = runs("a.md", "Just an ordinary sentence.");
    assert_eq!(
        out,
        vec![("Just an ordinary sentence.".to_string(), Token::Normal)]
    );
}
