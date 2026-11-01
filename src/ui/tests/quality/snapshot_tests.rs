//! Review artifacts preserve the same reference colours the quality gate measures.

use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Modifier},
};

use super::snapshot;

#[test]
fn svg_resolves_defaults_before_reversing_and_resolves_indexed_colours() {
    let mut buffer = Buffer::empty(Rect::new(0, 0, 2, 1));
    buffer[(0, 0)]
        .set_symbol("A")
        .set_style(ratatui::style::Style::default().add_modifier(Modifier::REVERSED));
    buffer[(1, 0)]
        .set_symbol("B")
        .set_fg(Color::Indexed(196))
        .set_bg(Color::Black);
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("review.svg");
    snapshot::save(&buffer, "Reference colours", &path).unwrap();
    let svg = std::fs::read_to_string(path).unwrap();
    assert!(
        svg.contains("x=\"0\" y=\"0\" width=\"10\" height=\"20\" fill=\"#ffffff\""),
        "{svg}"
    );
    assert!(svg.contains("x=\"0\" y=\"16\" fill=\"#000000\">A"), "{svg}");
    assert!(svg.contains("fill=\"#ff0000\">B"), "{svg}");
}

#[test]
fn svg_backgrounds_do_not_paint_over_wide_glyphs() {
    let mut buffer = Buffer::empty(Rect::new(0, 0, 3, 1));
    buffer[(0, 0)].set_symbol("界");
    buffer[(2, 0)].set_symbol("A");
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("wide.svg");
    snapshot::save(&buffer, "Wide glyph", &path).unwrap();
    let svg = std::fs::read_to_string(path).unwrap();
    assert!(
        svg.rfind("<rect").unwrap() < svg.find("<text").unwrap(),
        "{svg}"
    );
}

#[test]
fn svg_preserves_the_modifiers_that_make_hotkeys_distinct() {
    let mut buffer = Buffer::empty(Rect::new(0, 0, 1, 1));
    buffer[(0, 0)]
        .set_symbol("K")
        .set_style(ratatui::style::Style::default().add_modifier(
            Modifier::BOLD | Modifier::ITALIC | Modifier::UNDERLINED | Modifier::CROSSED_OUT,
        ));
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("modifiers.svg");
    snapshot::save(&buffer, "Hotkey modifiers", &path).unwrap();
    let svg = std::fs::read_to_string(path).unwrap();
    assert!(svg.contains("font-weight=\"bold\""), "{svg}");
    assert!(svg.contains("font-style=\"italic\""), "{svg}");
    assert!(
        svg.contains("text-decoration=\"underline line-through\""),
        "{svg}"
    );
}

#[test]
fn svg_keeps_a_wide_glyphs_background_across_its_continuation_cell() {
    let mut buffer = Buffer::empty(Rect::new(0, 0, 2, 1));
    buffer[(0, 0)]
        .set_symbol("界")
        .set_fg(Color::White)
        .set_bg(Color::Rgb(0, 0, 128));
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("wide-background.svg");
    snapshot::save(&buffer, "Wide background", &path).unwrap();
    let svg = std::fs::read_to_string(path).unwrap();
    assert!(
        svg.contains("width=\"20\" height=\"20\" fill=\"#000080\""),
        "{svg}"
    );
    assert!(!svg.contains("fill=\"#000000\""), "{svg}");
}
