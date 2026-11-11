//! Purpose: The half-transparent shadow Far drops behind a dialog or a
//! drop-down: the cells keep their characters and are darkened in place,
//! clipped to the screen.

use ratatui::{layout::Rect, style::Color, Frame};

use super::super::palette;

/// How far the drop shadow is offset. Two columns and one row is the ratio a
/// terminal cell has, so the shadow reads as square.
const SHADOW_X: u16 = 2;
const SHADOW_Y: u16 = 1;

/// The shadow Far drops behind a dialog, clipped to the screen.
///
/// Half-transparent rather than black: the cells keep their own characters and
/// are darkened in place, so the panel underneath still reads through the
/// shadow the way a real one would. Blanking them instead cuts a black hole in
/// the listing, which says "something is missing here" rather than "something
/// is in front of this".
///
/// Drawn before the dialog and never over it: the two rectangles overlap, and
/// painting the shadow second would darken the dialog's own right edge.
pub(in crate::ui) fn render_shadow(f: &mut Frame<'_>, area: Rect, screen: Rect) {
    let x = area.x + SHADOW_X;
    let y = area.y + SHADOW_Y;
    if x >= screen.right() || y >= screen.bottom() {
        return;
    }
    let width = area.width.min(screen.right() - x);
    let height = area.height.min(screen.bottom() - y);

    let dim = |colour: Color| match colour {
        Color::Rgb(red, green, blue) => Color::Rgb(
            shadowed_component(red),
            shadowed_component(green),
            shadowed_component(blue),
        ),
        // A named colour has no components to scale, so it goes to the
        // shadow's own darkness rather than being left at full brightness.
        _ => palette::SHADOW,
    };

    let buffer = f.buffer_mut();
    for row in y..y + height {
        for column in x..x + width {
            let cell = &mut buffer[(column, row)];
            let (fg, bg) = (cell.fg, cell.bg);
            cell.set_fg(dim(fg)).set_bg(dim(bg));
        }
    }
}

/// One RGB component of a shadowed cell: three eighths of it survive.
pub(super) fn shadowed_component(value: u8) -> u8 {
    /// How much of the original colour survives, out of eight.
    const KEPT: u16 = 3;
    // At most 255 * 3 / 8 = 95, so the saturating fallback is never reached.
    u8::try_from(u16::from(value) * KEPT / 8).unwrap_or(u8::MAX)
}
