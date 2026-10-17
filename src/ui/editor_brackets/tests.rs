//! Purpose: Assert which cells the matching-delimiter highlight paints.

use super::*;
use crate::models::TabSize;
use ratatui::{backend::TestBackend, Terminal};
use std::path::PathBuf;

const WIDTH: u16 = 12;
const HEIGHT: u16 = 3;

fn editor(text: &str, line: usize, column: usize) -> Editor {
    let mut editor = Editor::from_content("t.rs".into(), PathBuf::from("t.rs"), text);
    editor.cursor_line = line;
    editor.cursor_col = column;
    editor
}

/// The cells painted with the highlight, as (x, y), drawing from `top`
/// with `skip` columns scrolled off the left.
fn highlighted(editor: &Editor, top: usize, skip: usize) -> Vec<(u16, u16)> {
    let mut terminal = Terminal::new(TestBackend::new(WIDTH, HEIGHT)).unwrap();
    terminal
        .draw(|frame| render(frame, editor, frame.area(), top, Cells::new(skip)))
        .unwrap();
    let buffer = terminal.backend().buffer();
    let mut cells = Vec::new();
    for y in 0..HEIGHT {
        for x in 0..WIDTH {
            if buffer[(x, y)].bg == Color::LightYellow {
                cells.push((x, y));
            }
        }
    }
    cells
}

#[test]
fn every_bracket_kind_highlights_itself_and_its_partner() {
    for text in ["a(bc)d", "a[bc]d", "a{bc}d"] {
        assert_eq!(
            highlighted(&editor(text, 0, 1), 0, 0),
            [(1, 0), (4, 0)],
            "{text}"
        );
    }
}

#[test]
fn the_highlight_is_bold_black_on_light_yellow() {
    let mut terminal = Terminal::new(TestBackend::new(WIDTH, HEIGHT)).unwrap();
    let editor = editor("()", 0, 0);
    terminal
        .draw(|frame| render(frame, &editor, frame.area(), 0, Cells::new(0)))
        .unwrap();
    let cell = &terminal.backend().buffer()[(0, 0)];
    assert_eq!(cell.fg, Color::Black);
    assert!(cell.modifier.contains(Modifier::BOLD));
}

#[test]
fn a_closing_bracket_finds_its_opener_across_nesting() {
    // Caret on the last ')': the inner pair is skipped, the outer '(' matches.
    assert_eq!(
        highlighted(&editor("f(a(b)c)", 0, 7), 0, 0),
        [(1, 0), (7, 0)]
    );
}

#[test]
fn an_opening_bracket_finds_its_closer_across_nesting() {
    assert_eq!(
        highlighted(&editor("f(a(b)c)", 0, 1), 0, 0),
        [(1, 0), (7, 0)]
    );
}

#[test]
fn the_caret_just_after_a_bracket_selects_that_bracket() {
    // Column 5 is past the ')' at column 4, where typing leaves the caret.
    assert_eq!(highlighted(&editor("a(bc)", 0, 5), 0, 0), [(1, 0), (4, 0)]);
}

#[test]
fn a_pair_spanning_lines_is_found_in_both_directions() {
    let text = "{\n  x\n}";
    assert_eq!(highlighted(&editor(text, 0, 0), 0, 0), [(0, 0), (0, 2)]);
    assert_eq!(highlighted(&editor(text, 2, 0), 0, 0), [(0, 0), (0, 2)]);
}

#[test]
fn nothing_is_highlighted_without_a_bracket_at_or_before_the_caret() {
    assert!(highlighted(&editor("abc(d)", 0, 1), 0, 0).is_empty());
    assert!(highlighted(&editor("abc", 0, 0), 0, 0).is_empty());
}

#[test]
fn an_unmatched_bracket_highlights_nothing() {
    assert!(highlighted(&editor("a(b", 0, 1), 0, 0).is_empty());
    assert!(highlighted(&editor("a)b", 0, 1), 0, 0).is_empty());
}

#[test]
fn a_caret_on_a_line_past_the_end_highlights_nothing() {
    assert!(highlighted(&editor("()", 5, 0), 0, 0).is_empty());
}

#[test]
fn a_partner_scrolled_off_vertically_is_not_drawn() {
    // Lines 0..=3; the view starts at line 1, so the '{' on line 0 is gone.
    let text = "{\na\nb\n}";
    assert_eq!(highlighted(&editor(text, 3, 0), 1, 0), [(0, 2)]);
    // A three-row view from line 0 never reaches the '}' on line 3.
    assert_eq!(highlighted(&editor(text, 0, 0), 0, 0), [(0, 0)]);
}

#[test]
fn horizontal_scroll_shifts_the_highlight_and_drops_what_falls_off() {
    let text = "(abcdefghijklmnop)";
    // Two columns scrolled off: the '(' is gone, the ')' at 17 is past the
    // twelve visible cells.
    assert!(highlighted(&editor(text, 0, 0), 0, 2).is_empty());
    // Eight scrolled off: the ')' lands on cell 17 - 8 = 9.
    assert_eq!(highlighted(&editor(text, 0, 17), 0, 8), [(9, 0)]);
}

#[test]
fn a_tab_before_the_bracket_moves_its_cell_to_the_tab_stop() {
    let mut tabbed = editor("\t()", 0, 1);
    tabbed.tab_size = TabSize::new(4);
    assert_eq!(highlighted(&tabbed, 0, 0), [(4, 0), (5, 0)]);
}

#[test]
fn columns_count_characters_not_bytes_in_both_directions() {
    // Two-byte letters before and inside the pair: columns are characters.
    let text = "éé(ü)";
    assert_eq!(highlighted(&editor(text, 0, 2), 0, 0), [(2, 0), (4, 0)]);
    assert_eq!(highlighted(&editor(text, 0, 4), 0, 0), [(2, 0), (4, 0)]);
}

#[test]
fn xml_tag_pair_highlights_both_opening_and_closing_tags_in_bright_red() {
    let line = "  <fb-contrib.version>7.7.4</fb-contrib.version>";
    let mut ed = Editor::from_content("pom.xml".into(), PathBuf::from("pom.xml"), line);
    ed.cursor_line = 0;
    ed.cursor_col = 44; // Inside `</fb-contrib.version>`
    let mut terminal = Terminal::new(TestBackend::new(60, 1)).unwrap();
    terminal
        .draw(|frame| render(frame, &ed, frame.area(), 0, Cells::new(0)))
        .unwrap();
    let buffer = terminal.backend().buffer();
    let red: Vec<u16> = (0..60)
        .filter(|&x| buffer[(x, 0)].fg == super::super::palette::CODE_URL)
        .collect();
    let expected: Vec<u16> = (2..22).chain(27..48).collect();
    assert_eq!(red, expected);
}

#[test]
fn multiline_xml_opening_tag_with_attributes_pairs_with_closing_tag() {
    let xml = "<project xmlns=\"http://maven.apache.org/POM/4.0.0\"\n         xsi:schemaLocation=\"x\">\n</project>";
    let mut ed = Editor::from_content("pom.xml".into(), PathBuf::from("pom.xml"), xml);
    ed.cursor_line = 0;
    ed.cursor_col = 0;
    let spans = xml::matching_xml_pair(&ed).unwrap();
    assert_eq!(
        spans[0],
        xml::TagSpan {
            line: 0,
            start_col: 0,
            end_col: 8
        }
    );
    assert_eq!(
        spans[1],
        xml::TagSpan {
            line: 2,
            start_col: 0,
            end_col: 10
        }
    );
}
