//! Purpose: Where the Sort by menu sits and what a click on it lands on: the
//! box sized to its rows over the panel it sorts, dragged and clamped to the
//! screen, and the lines it scrolls through with the rule between them.

use ratatui::layout::Rect;

use super::super::{far_block, Cells};
use super::{GAP, INSET, PAD};
use crate::models::{PanelSide, SortMenu, SortRow};

/// The row under a click, as a cursor position — the rule between the modes
/// and the order switch is drawn on a line of its own and names no row.
pub fn row_at(
    screen: Rect,
    menu: &SortMenu,
    offset: (i32, i32),
    column: u16,
    row: u16,
) -> Option<usize> {
    let body = far_block().inner(place(screen, menu.side, offset)?);
    if !body.contains((column, row).into()) {
        return None;
    }
    let line = first_line(menu, body.height) + usize::from(row - body.y);
    let clicked = lines().get(line).copied().flatten()?;
    SortRow::ALL.iter().position(|row| *row == clicked)
}

/// Where the box goes: sized to its rows, dropped over the panel it sorts a
/// little in from that panel's left edge, centred vertically, then shifted by
/// however far it has been dragged — clamped so it can never leave the screen.
///
/// `None` when the screen cannot hold even the frame.
pub(in crate::ui) fn place(screen: Rect, side: PanelSide, offset: (i32, i32)) -> Option<Rect> {
    let width = u16::try_from(interior_width() + 2)
        .unwrap_or(u16::MAX)
        .min(screen.width);
    let height = u16::try_from(lines().len() + 2)
        .unwrap_or(u16::MAX)
        .min(screen.height);
    if width < 4 || height < 3 {
        return None;
    }
    let panel_left = match side {
        PanelSide::Left => 0,
        PanelSide::Right => screen.width / 2,
    };
    let anchored_x =
        i32::from(screen.x) + i32::from((panel_left + INSET).min(screen.width - width));
    let centred_y = i32::from(screen.y) + i32::from(screen.height - height) / 2;
    let max_x = i32::from(screen.right() - width);
    let max_y = i32::from(screen.bottom() - height);
    Some(Rect {
        x: u16::try_from((anchored_x + offset.0).clamp(i32::from(screen.x), max_x)).ok()?,
        y: u16::try_from((centred_y + offset.1).clamp(i32::from(screen.y), max_y)).ok()?,
        width,
        height,
    })
}

/// The menu top to bottom: every row, with the rule — `None` — between the
/// modes and the order switch.
pub(super) fn lines() -> Vec<Option<SortRow>> {
    let mut lines: Vec<Option<SortRow>> = SortRow::ALL.iter().copied().map(Some).collect();
    lines.insert(SortRow::ALL.len() - 1, None);
    lines
}

/// The first line drawn: the list scrolls only as far as it must for the
/// cursor row to land on the body's last line.
pub(super) fn first_line(menu: &SortMenu, body_height: u16) -> usize {
    let cursor = lines()
        .iter()
        .position(|line| *line == Some(menu.row()))
        .unwrap_or(0);
    (cursor + 1).saturating_sub(usize::from(body_height))
}

/// The columns inside the frame: the padding, the tick and its blank, the
/// widest label, the gap, and the widest key.
fn interior_width() -> usize {
    let widest = SortRow::ALL
        .iter()
        .map(|row| Cells::of(row.label()).get() + GAP + Cells::of(&row.shortcut()).get())
        .max()
        .unwrap_or(0);
    PAD + 2 + widest + PAD
}

#[cfg(test)]
mod tests_side;
