//! Purpose: What sits under the mouse pointer, and what clicking it means.
//!
//! Pure geometry, kept apart from both the renderer and the event loop. A
//! click is only correct if the arithmetic that decides what was hit is the
//! same arithmetic that decided where to draw it — so the layout lives here,
//! in functions the renderer calls too, rather than being written twice and
//! left to drift.
//!
//! Everything here is a function of the screen size and the app's state. There
//! is no map of rectangles recorded during rendering: that would be state to
//! keep in step, and a stale entry means a click landing on the wrong thing
//! rather than on nothing, which is the harder failure to notice.

use ratatui::layout::Rect;

use crate::models::{PanelSide, PanelsShown};

mod clicks;
mod dialog;

pub(crate) use clicks::{PanelClicks, PanelRowClick, Wheel};
pub use dialog::{DialogMouse, DialogTarget};

/// The `count` cells of the F-key bar, in columns, for a terminal this wide.
///
/// The remainder is spread over the leftmost cells rather than dumped on the
/// last one — that is what keeps an 85-column terminal from ending in a cell
/// five columns wider than its neighbours, and it is why this cannot simply be
/// `column / (width / 12)`. A bar of no keys has no cells.
#[must_use]
pub fn fkey_widths(width: u16, count: usize) -> Vec<u16> {
    if count == 0 {
        return Vec::new();
    }
    let total = usize::from(width);
    let base = total / count;
    let remainder = total % count;
    (0..count)
        .map(|index| u16::try_from(base + usize::from(index < remainder)).unwrap_or(u16::MAX))
        .collect()
}

/// Which function key covers `column` on a bar `width` wide, one-based.
#[must_use]
pub fn fkey_at(width: u16, count: usize, column: u16) -> Option<u8> {
    let mut left = 0u16;
    for (index, cell) in fkey_widths(width, count).into_iter().enumerate() {
        let right = left.saturating_add(cell);
        if column < right {
            return u8::try_from(index + 1).ok();
        }
        left = right;
    }
    None
}

/// What the pointer is over.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Target {
    /// One of the function keys on the bottom bar, one-based.
    FunctionKey(u8),
    /// An entry row in a panel: which side, and how far down the visible rows.
    PanelRow { side: PanelSide, row: usize },
    /// A panel, but not one of its entry rows — its frame, header or footer.
    /// Clicking still makes that side active, which is what a user reaching
    /// for the other panel means.
    Panel(PanelSide),
    /// The command line.
    CommandLine,
    /// Nothing that answers to a click.
    Nothing,
}

/// Rows the screen spends above the panels: Far's menu bar.
const MENU_BAR_ROWS: u16 = 1;

/// Rows the screen spends below the panels: the command line and the key bar.
const BOTTOM_CHROME_ROWS: u16 = 2;

/// Rows inside a panel's frame that are not entries: the column header, the
/// rule above the info row, and the info row itself.
const PANEL_NON_ENTRY_ROWS: u16 = 3;

/// What is at `(column, row)` on a screen of size `area`.
///
/// Mirrors [`crate::ui::ui`]'s layout: the menu bar takes the first row, panels
/// fill the space above the last two rows and split half and half — or one
/// panel takes it all, when `shown` says the other is off screen — and each
/// panel spends its first two and last three rows on chrome.
#[must_use]
pub fn hit(area: Rect, column: u16, row: u16, keys: usize, shown: PanelsShown) -> Target {
    let Some((column, row)) = local(area, column, row) else {
        return Target::Nothing;
    };

    let last = area.height.saturating_sub(1);
    if row == last {
        return fkey_at(area.width, keys, column).map_or(Target::Nothing, Target::FunctionKey);
    }
    // `saturating_add`: a mouse event carrying `u16::MAX` is not a normal
    // event, but overflowing on it in a debug build is not the right answer.
    if row.saturating_add(1) == last {
        return Target::CommandLine;
    }

    if row < MENU_BAR_ROWS {
        return Target::Nothing;
    }
    let panel_row = row - MENU_BAR_ROWS;
    let panels_height = area
        .height
        .saturating_sub(MENU_BAR_ROWS + BOTTOM_CHROME_ROWS);
    if panel_row >= panels_height {
        return Target::Nothing;
    }

    let side = panel_side(area.width, column, shown);
    panel_target(side, panels_height, panel_row)
}

/// `(column, row)` relative to `area`'s origin, or `None` when the point is
/// outside it — which a screen with no rows or no columns always is.
fn local(area: Rect, column: u16, row: u16) -> Option<(u16, u16)> {
    let column = column.checked_sub(area.x)?;
    let row = row.checked_sub(area.y)?;
    (column < area.width && row < area.height).then_some((column, row))
}

/// Which panel owns `column`. With both on screen they split the width the
/// way `Layout` does: the left takes the floor, so an odd terminal gives the
/// extra column to the right panel. A panel alone on screen owns every column
/// — a click can never reach the one that is not drawn.
fn panel_side(width: u16, column: u16, shown: PanelsShown) -> PanelSide {
    match shown {
        PanelsShown::LeftOnly => PanelSide::Left,
        PanelsShown::RightOnly => PanelSide::Right,
        PanelsShown::Both => {
            if column < width / 2 {
                PanelSide::Left
            } else {
                PanelSide::Right
            }
        }
    }
}

fn panel_target(side: PanelSide, panels_height: u16, panel_row: u16) -> Target {
    // Inside the frame: row 0 is the border, row 1 the column header, then the
    // entries, then the rule, the info row and the bottom border.
    let first_entry = 2;
    let last_entry = panels_height.saturating_sub(1 + PANEL_NON_ENTRY_ROWS);
    if panel_row >= first_entry && panel_row <= last_entry {
        return Target::PanelRow {
            side,
            row: usize::from(panel_row - first_entry),
        };
    }
    Target::Panel(side)
}

#[cfg(test)]
mod tests;
