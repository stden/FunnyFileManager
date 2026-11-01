//! Validate readable glyphs, required captions and interactive geometry.

use ratatui::{buffer::Buffer, layout::Rect};
use unicode_width::UnicodeWidthStr;

use super::Case;

pub(super) fn validate(buffer: &Buffer, case: &Case, width: u16, height: u16) {
    let context = format!("{} at {width}x{height}", case.name);
    let screen = buffer
        .content
        .chunks(usize::from(width))
        .map(|row| {
            row.iter()
                .map(ratatui::buffer::Cell::symbol)
                .collect::<String>()
        })
        .collect::<Vec<_>>()
        .join("\n");
    required_text(&screen, &case.required, &context);
    if width >= 80 {
        required_text(&screen, &case.wide_required, &context);
    }
    readable_cells(buffer, &context);
    interactive_geometry(case, Rect::new(0, 0, width, height), &context);
}

fn required_text(screen: &str, required: &[&str], context: &str) {
    for text in required {
        assert!(
            screen.contains(text),
            "{context}: missing {text:?}\n{screen}"
        );
    }
}

fn readable_cells(buffer: &Buffer, context: &str) {
    for y in buffer.area.top()..buffer.area.bottom() {
        for x in buffer.area.left()..buffer.area.right() {
            let symbol = buffer[(x, y)].symbol();
            assert!(
                !symbol.chars().any(|ch| ch.is_control() || ch == '\u{fffd}'),
                "{context}: unreadable cell {x},{y}: {symbol:?}"
            );
            let width = UnicodeWidthStr::width(symbol);
            assert!(width <= 2, "{context}: oversized cell {x},{y}: {symbol:?}");
            if width == 2 {
                assert!(
                    x + 1 < buffer.area.right(),
                    "{context}: cut wide glyph at {x},{y}"
                );
                assert!(
                    buffer[(x + 1, y)].symbol().trim().is_empty(),
                    "{context}: wide glyph overlaps the next cell at {x},{y}"
                );
            }
        }
    }
}

fn interactive_geometry(case: &Case, screen: Rect, context: &str) {
    let tracker = &case.app.dialog_mouse;
    if let Some(area) = tracker.area {
        assert_eq!(
            area.intersection(screen),
            area,
            "{context}: dialog leaves the screen"
        );
    }
    for (position, &(_, left, right)) in tracker.buttons.iter().enumerate() {
        assert!(
            left < right && right <= screen.right(),
            "{context}: clipped button"
        );
        assert!(
            tracker.buttons_y < screen.bottom(),
            "{context}: buttons below the screen"
        );
        for &(_, next_left, next_right) in tracker.buttons.iter().skip(position + 1) {
            assert!(
                right <= next_left || next_right <= left,
                "{context}: overlapping button hit regions"
            );
        }
    }
}

#[test]
#[should_panic(expected = "missing \"Required\"")]
fn missing_essential_caption_is_rejected() {
    let buffer = Buffer::empty(Rect::new(0, 0, 20, 1));
    let case = super::plain("missing-caption", &["Required"]);
    validate(&buffer, &case, 20, 1);
}

#[test]
fn controls_and_replacement_marks_are_rejected() {
    for symbol in ["\u{1b}", "\u{fffd}"] {
        let mut buffer = Buffer::empty(Rect::new(0, 0, 2, 1));
        buffer[(0, 0)].set_symbol(symbol);
        let failure = std::panic::catch_unwind(|| readable_cells(&buffer, "invalid text"));
        assert!(failure.is_err(), "accepted {symbol:?}");
    }
}

#[test]
#[should_panic(expected = "cut wide glyph")]
fn clipped_wide_glyph_is_rejected() {
    let mut buffer = Buffer::empty(Rect::new(0, 0, 2, 1));
    buffer[(1, 0)].set_symbol("界");
    readable_cells(&buffer, "clipped glyph");
}

#[test]
#[should_panic(expected = "wide glyph overlaps")]
fn overlapping_wide_glyph_is_rejected() {
    let mut buffer = Buffer::empty(Rect::new(0, 0, 3, 1));
    buffer[(0, 0)].set_symbol("界");
    buffer[(1, 0)].set_symbol("x");
    readable_cells(&buffer, "overlapping glyph");
}

#[test]
#[should_panic(expected = "overlapping button hit regions")]
fn overlapping_buttons_are_rejected() {
    let mut case = super::plain("overlapping-buttons", &[]);
    case.app.dialog_mouse.buttons = vec![(0, 1, 5), (1, 4, 9)];
    case.app.dialog_mouse.buttons_y = 1;
    interactive_geometry(&case, Rect::new(0, 0, 20, 4), "overlapping buttons");
}

#[test]
fn wide_and_combining_graphemes_are_readable() {
    let mut buffer = Buffer::empty(Rect::new(0, 0, 8, 1));
    buffer.set_string(0, 0, "界e\u{301}", ratatui::style::Style::default());
    readable_cells(&buffer, "valid Unicode");
}
