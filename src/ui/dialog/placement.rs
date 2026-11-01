//! Purpose: Where a dialog sits: sized to its content, centred, shifted by
//! however far it was dragged and clamped to the screen — one computation
//! that the renderer and the mouse both call.

use ratatui::layout::Rect;

use super::super::Cells;
use super::button::{Button, BUTTON_GAP};
use super::row::Row;

/// Blank dialog surface around the frame, top/bottom and left/right.
///
/// Far insets the frame rather than running it to the edge, so the dialog
/// reads as a panel with a box on it rather than as a box with nothing around
/// it. Two columns to one row, because a cell is about twice as tall as wide.
const MARGIN_X: u16 = 2;
const MARGIN_Y: u16 = 1;

/// Rows the chrome spends: the margin twice, the frame twice, the rule above
/// the buttons, and the button row itself.
const CHROME_ROWS: u16 = MARGIN_Y * 2 + 2 + 1 + 1;

/// Columns the chrome spends: the margin twice, the frame twice, and one blank
/// inside the frame on each side.
const CHROME_COLUMNS: u16 = MARGIN_X * 2 + 2 + 2;

/// Where a dialog and its parts sit on screen.
///
/// Computed from the screen size and the dialog's own content, and by exactly
/// one function — [`place`] — which both the renderer and the mouse call. A
/// click is only correct if the arithmetic that decides what was hit is the
/// arithmetic that decided where to draw it; two copies would drift, and a
/// drifted copy sends the click to the wrong button rather than to nothing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::ui) struct Placement {
    /// The whole dialog, shadow excluded.
    pub area: Rect,
    /// Inside the frame: the body rows, then the rule, then the buttons.
    pub body: Rect,
    /// The row the buttons are drawn on.
    pub buttons_y: u16,
}

/// Size and centre a dialog for `screen`.
///
/// `None` when the screen cannot hold even the frame, which is the same
/// condition [`render`] gives up on.
pub(in crate::ui) fn place(
    screen: Rect,
    title: &str,
    rows: &[Row<'_>],
    buttons: &[Button<'_>],
    offset: (i32, i32),
) -> Option<Placement> {
    let (width, height) = dialog_size(screen, title, rows, buttons);
    // Centred, then shifted by however far the user has dragged it. The shift
    // is clamped so a dialog can never be dragged off the screen and lost.
    let centred_x = i32::from(screen.x) + i32::from(screen.width.saturating_sub(width)) / 2;
    let centred_y = i32::from(screen.y) + i32::from(screen.height.saturating_sub(height)) / 2;
    let max_x = i32::from(screen.right().saturating_sub(width));
    let max_y = i32::from(screen.bottom().saturating_sub(height));
    let area = Rect {
        x: u16::try_from((centred_x + offset.0).clamp(i32::from(screen.x), max_x.max(0))).ok()?,
        y: u16::try_from((centred_y + offset.1).clamp(i32::from(screen.y), max_y.max(0))).ok()?,
        width,
        height,
    };

    let framed = frame_area(area);
    if framed.height < 3 || framed.width < 4 {
        return None;
    }
    let body = Rect {
        x: framed.x + 1,
        y: framed.y + 1,
        width: framed.width.saturating_sub(2),
        height: framed.height.saturating_sub(2),
    };
    Some(Placement {
        area,
        body,
        buttons_y: body.y + body.height.saturating_sub(1),
    })
}

fn dialog_size(screen: Rect, title: &str, rows: &[Row<'_>], buttons: &[Button<'_>]) -> (u16, u16) {
    let button_row: Cells = buttons
        .iter()
        .map(|button| button.width() + Cells::of(BUTTON_GAP))
        .sum();

    let widest = rows
        .iter()
        .map(Row::width)
        .chain(std::iter::once(button_row))
        .chain(std::iter::once(Cells::of(title) + Cells::new(4)))
        .max()
        .unwrap_or_default();

    let width = u16::try_from(widest.get())
        .unwrap_or(u16::MAX)
        .saturating_add(CHROME_COLUMNS)
        .min(screen.width);
    let height = u16::try_from(rows.len())
        .unwrap_or(u16::MAX)
        .saturating_add(CHROME_ROWS)
        .min(screen.height);

    (width, height)
}

pub(super) fn frame_area(area: Rect) -> Rect {
    Rect {
        x: area.x + MARGIN_X,
        y: area.y + MARGIN_Y,
        width: area.width.saturating_sub(MARGIN_X * 2),
        height: area.height.saturating_sub(MARGIN_Y * 2),
    }
}

/// Cells available for title text after frame insets and surrounding blanks.
pub(in crate::ui) fn title_width(area: Rect) -> Cells {
    Cells::columns(frame_area(area).width.saturating_sub(4))
}
