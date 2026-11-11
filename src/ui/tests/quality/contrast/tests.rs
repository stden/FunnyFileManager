//! The checker rejects faint text and exempts only declared non-text states.

use ratatui::{
    buffer::Cell,
    layout::Rect,
    style::{Color, Modifier, Style},
};

use super::*;
use crate::ui::palette;
use crate::ui::tests::quality::colors::rgb;

fn single(foreground: Color, background: Color) -> Buffer {
    let mut buffer = Buffer::empty(Rect::new(0, 0, 1, 1));
    buffer[(0, 0)]
        .set_symbol("A")
        .set_fg(foreground)
        .set_bg(background);
    buffer
}

fn assert_readable(buffer: &Buffer) {
    let failures = check(buffer);
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

#[test]
fn luminance_reference_values_and_threshold_are_not_rounded() {
    assert_eq!(contrast([0; 3], [255; 3]), 21.0);
    assert_eq!(contrast([75; 3], [75; 3]), 1.0);
    assert!(contrast([118; 3], [255; 3]) >= MINIMUM_TEXT_CONTRAST);
    assert!(contrast([119; 3], [255; 3]) < MINIMUM_TEXT_CONTRAST);
    assert_readable(&single(Color::Rgb(118, 118, 118), Color::White));
    assert!(
        !check(&single(Color::Rgb(119, 119, 119), Color::White)).is_empty(),
        "text below 4.5:1 must fail"
    );
    let boundary = contrast([129, 115, 122], [255; 3]);
    assert!(boundary > 4.4999 && boundary < MINIMUM_TEXT_CONTRAST);
    assert!(
        !check(&single(Color::Rgb(129, 115, 122), Color::White)).is_empty(),
        "rounding cannot make text pass"
    );
}

#[test]
fn reference_terminal_colours_include_rgb_ansi_indexed_and_reset() {
    assert_eq!(rgb(Color::Rgb(10, 20, 30), [0; 3]), [10, 20, 30]);
    assert_eq!(rgb(Color::Reset, [9, 8, 7]), [9, 8, 7]);
    assert_eq!(rgb(Color::Black, [255; 3]), [0; 3]);
    assert_eq!(rgb(Color::White, [0; 3]), [255; 3]);
    assert_eq!(rgb(Color::Indexed(0), [255; 3]), [0; 3]);
    assert_eq!(rgb(Color::Indexed(16), [255; 3]), [0; 3]);
    assert_eq!(rgb(Color::Indexed(231), [0; 3]), [255; 3]);
    assert_eq!(rgb(Color::Indexed(232), [255; 3]), [8; 3]);
    assert_eq!(rgb(Color::Indexed(255), [0; 3]), [238; 3]);
}

#[test]
fn scanner_reports_bad_cells_and_checks_dim_and_reversed_text() {
    let mut buffer = single(Color::Rgb(120, 120, 120), Color::Rgb(130, 130, 130));
    buffer[(0, 0)].modifier = Modifier::DIM | Modifier::REVERSED;
    let failures = check(&buffer);
    assert_eq!(failures.len(), 1);
    assert!(failures[0].contains("(0,0)"));
    assert!(failures[0].contains("requires 4.5:1"));
    let mut cell = Cell::new("A");
    cell.set_fg(Color::Rgb(10, 20, 30))
        .set_bg(Color::Rgb(40, 50, 60));
    cell.modifier = Modifier::REVERSED;
    assert_eq!(effective(&cell), ([40, 50, 60], [10, 20, 30]));
}

#[test]
fn no_text_colour_is_exempt_but_decorative_glyphs_are() {
    let buffer = single(palette::DISABLED_FG, palette::FIELD_BG);
    assert!(
        !check(&buffer).is_empty(),
        "a colour cannot declare text inactive"
    );
    for symbol in [" ", "═", "▄"] {
        let mut buffer = single(Color::Black, Color::Black);
        buffer[(0, 0)].set_symbol(symbol);
        assert_readable(&buffer);
    }
}

#[test]
fn unavailable_controls_are_readable_without_actionable_accents() {
    for background in [
        palette::DIALOG_BG,
        palette::WARNING_BG,
        palette::FIELD_BG,
        palette::BACKGROUND,
        palette::GRAY_BACKGROUND,
        palette::BLACK_BACKGROUND,
        palette::HIGHLIGHT_BG,
    ] {
        let style = palette::disabled_style(
            Style::default()
                .bg(background)
                .add_modifier(Modifier::BOLD | Modifier::UNDERLINED | Modifier::DIM),
        );
        assert_readable(&single(style.fg.unwrap(), background));
        assert!(style.add_modifier.contains(Modifier::ITALIC));
        assert!(!style
            .add_modifier
            .intersects(Modifier::BOLD | Modifier::UNDERLINED | Modifier::DIM));
    }
}

#[test]
fn essential_palette_text_pairs_meet_the_readability_budget() {
    let pairs = [
        ("cursor", palette::CURSOR_FG, palette::CURSOR_BG),
        ("marked cursor", palette::MARKED, palette::CURSOR_BG),
        ("help", palette::HELP_FG, palette::HELP_BG),
        ("menu", palette::MENU_FG, palette::MENU_BG),
        ("field", palette::FIELD_FG, palette::FIELD_BG),
        ("comment", palette::CODE_COMMENT, palette::BACKGROUND),
        ("URL", palette::CODE_URL, palette::BACKGROUND),
        ("URI", palette::CODE_URI_FG, palette::CODE_URI_BG),
    ];
    let failures: Vec<_> = pairs
        .into_iter()
        .flat_map(|(name, foreground, background)| {
            check(&single(foreground, background))
                .into_iter()
                .map(move |failure| format!("{name}: {failure}"))
        })
        .collect();
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}
