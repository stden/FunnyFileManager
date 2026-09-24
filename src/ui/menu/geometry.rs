//! Purpose: Where the menu bar's headings and the open drop-down sit on
//! screen — pure functions of the menu and the screen, so a click asks the
//! same question the renderer answered.

use ratatui::layout::Rect;

use super::super::Cells;
use super::{GAP, MIN_SHORTCUT_GAP, PAD};
use crate::menu::{Entry, Heading, Menu};

/// Where the open drop-down sits: under its heading, clamped to the screen.
///
/// A pure function of the menu and the screen, so the mouse can ask the same
/// question the renderer answered.
pub(in crate::ui) fn drop_down_area(menu: &Menu, bar: Rect, screen: Rect) -> Option<Rect> {
    let widest = menu_width(menu);
    let width = u16::try_from(widest.get())
        .unwrap_or(u16::MAX)
        .min(screen.width);
    let height = u16::try_from(menu.entries().len())
        .unwrap_or(u16::MAX)
        .saturating_add(2)
        .min(screen.height.saturating_sub(bar.height));
    if width < 4 || height < 3 {
        return None;
    }

    // Under its own heading, pulled left if it would run off the edge.
    let x = bar
        .x
        .saturating_add(heading_column(menu.heading))
        .min(screen.right().saturating_sub(width));
    Some(Rect {
        x,
        y: bar.y + bar.height,
        width,
        height,
    })
}

fn menu_width(menu: &Menu) -> Cells {
    menu.entries()
        .iter()
        .map(|entry| match entry {
            Entry::Item {
                label, shortcut, ..
            } => {
                Cells::of(&label.replace('&', ""))
                    + Cells::of(shortcut)
                    + Cells::new(PAD * 2 + 4 + MIN_SHORTCUT_GAP)
            }
            Entry::Separator => Cells::ZERO,
        })
        .max()
        .unwrap_or_default()
}

/// The column a heading starts at, counted the way [`render_bar`] draws them.
fn heading_column(wanted: Heading) -> u16 {
    let mut column = 0usize;
    for heading in Heading::ALL {
        column += Cells::of(GAP).get();
        if heading == wanted {
            break;
        }
        column += Cells::of(&heading.label().replace('&', "")).get();
    }
    u16::try_from(column).unwrap_or(0)
}

/// The heading at `column` on the bar, if any.
///
/// The same arithmetic [`render_bar`] draws with, so a click lands on the
/// heading the user is pointing at.
#[must_use]
pub fn heading_at(bar: Rect, column: u16) -> Option<Heading> {
    let mut left = bar.x;
    for heading in Heading::ALL {
        left = left.saturating_add(u16::try_from(Cells::of(GAP).get()).unwrap_or(0));
        let width = u16::try_from(Cells::of(&heading.label().replace('&', "")).get()).unwrap_or(0);
        if (left..left.saturating_add(width)).contains(&column) {
            return Some(heading);
        }
        left = left.saturating_add(width);
    }
    None
}

/// The drop-down row at `(column, row)`, if the click is inside one.
#[must_use]
pub fn menu_row_at(menu: &Menu, screen: Rect, column: u16, row: u16) -> Option<usize> {
    let bar = Rect {
        height: 1,
        ..screen
    };
    let area = drop_down_area(menu, bar, screen)?;
    let body = Rect {
        x: area.x + 1,
        y: area.y + 1,
        width: area.width.saturating_sub(2),
        height: area.height.saturating_sub(2),
    };
    if column < body.x || column >= body.x + body.width || row >= body.bottom() {
        return None;
    }
    let first = menu
        .row
        .saturating_sub(usize::from(body.height.saturating_sub(1)));
    let index = first + usize::from(row.checked_sub(body.y)?);
    menu.entries()
        .get(index)
        .filter(|entry| entry.is_selectable())
        .map(|_| index)
}
