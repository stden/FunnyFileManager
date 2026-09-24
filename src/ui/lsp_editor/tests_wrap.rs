//! Purpose: Pin how a server's detail wraps: each line filled to its last
//! cell, the rest indented under the value, and progress on a screen
//! narrower than the indent.

use super::*;

/// Each line of `wrap`'s output as the text it shows.
fn wrapped(pieces: &[&str], indent: usize, width: usize) -> Vec<String> {
    let style = ratatui::style::Style::default().fg(palette::CODE_STRING);
    let pieces = pieces
        .iter()
        .map(|text| ((*text).to_owned(), style))
        .collect();
    super::super::detail::wrap(pieces, indent, crate::ui::Cells::new(width))
        .iter()
        .map(|line| {
            line.spans
                .iter()
                .map(|span| span.content.as_ref())
                .collect()
        })
        .collect()
}

/// A value fills its line to the last cell and goes on under itself; a piece
/// cut in two keeps its colour on both lines.
#[test]
fn wrap_fills_each_line_and_indents_the_rest() {
    assert_eq!(wrapped(&["abcdef"], 2, 6), ["abcdef"]);
    assert_eq!(
        wrapped(&["abc", "defghijklm"], 2, 6),
        ["abcdef", "  ghij", "  klm"]
    );
    let style = ratatui::style::Style::default().fg(palette::CODE_STRING);
    let lines = super::super::detail::wrap(
        vec![("abcdefgh".to_owned(), style)],
        2,
        crate::ui::Cells::new(6),
    );
    for line in &lines {
        let text = line
            .spans
            .iter()
            .rev()
            .find(|span| !span.content.trim().is_empty());
        assert_eq!(text.map(|span| span.style), Some(style), "{line:?}");
    }
}

/// On a screen narrower than the indent every line still shows a character:
/// a wide one overhangs rather than leave an empty line, and a combining mark
/// stays with the letter it belongs to.
#[test]
fn wrap_narrower_than_its_indent_still_makes_progress() {
    assert_eq!(wrapped(&["中文字"], 2, 3), ["中文", "  字"]);
    assert_eq!(
        wrapped(&["abc\u{301}de"], 2, 1),
        ["abc\u{301}", "  d", "  e"]
    );
}
