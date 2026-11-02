//! Purpose: Far's Alt+F1 / Alt+F2 "Change drive" dialog.
//!
//! Far draws it as a menu with columns, not as a form: a boxed table of the
//! machine's filesystems — letter, kind, size, free — and under a rule
//! whatever else can be opened as a panel. The inner frame is drawn from
//! characters here rather than nested `Block`s, because the rule between the
//! two sections has to close the column separators above it (`┴`), which a
//! border cannot express.

mod table;

use ratatui::{
    layout::Rect,
    style::Style,
    widgets::{Clear, Paragraph},
    Frame,
};

use super::panel::abbreviate_bytes;
use super::{palette, Cells};
use crate::models::{DriveRow, PanelSide, RowKind};
use table::body;

/// Spaces either side of a column's text, as Far pads its cells.
const PAD: usize = 1;

/// Written into the top border, as Far titles it — the same for either panel,
/// because the highlighted panel is already the one that answers.
const TITLE: &str = " Change drive ";

/// Written into the bottom border. Only the keys the dialog actually handles:
/// a legend promising keys that do nothing is worse than no legend.
const FOOTER: &str = " Enter  Esc ";

/// The three columns of a disk row, already rendered to text.
struct Columns {
    label: String,
    total: String,
    free: String,
}

/// Column text for a disk row, or `None` for anything else.
///
/// A drive that will not report its size — an empty bay, a share that has gone
/// away — leaves those columns blank rather than claiming zero bytes.
fn columns(kind: &RowKind) -> Option<Columns> {
    let RowKind::Disk {
        label,
        device,
        space,
    } = kind
    else {
        return None;
    };
    let name = device.label();
    Some(Columns {
        label: if name.is_empty() {
            label.clone()
        } else {
            format!("{label} {name}")
        },
        total: space.map_or_else(String::new, |s| abbreviate_bytes(s.total)),
        free: space.map_or_else(String::new, |s| abbreviate_bytes(s.available)),
    })
}

/// How wide each column has to be for every row to fit without clipping.
struct Widths {
    label: usize,
    total: usize,
    free: usize,
    /// The widest plugin row, hotkey and padding included.
    plugin: usize,
    disks: usize,
}

impl Widths {
    fn of(rows: &[DriveRow]) -> Self {
        let mut widths = Self {
            label: 0,
            total: 0,
            free: 0,
            plugin: 0,
            disks: 0,
        };
        for row in rows {
            if let Some(cols) = columns(&row.kind) {
                widths.disks += 1;
                widths.label = widths.label.max(Cells::of(&cols.label).get());
                widths.total = widths.total.max(Cells::of(&cols.total).get());
                widths.free = widths.free.max(Cells::of(&cols.free).get());
            } else if let RowKind::Plugin { name } = &row.kind {
                widths.plugin = widths.plugin.max(Cells::of(name).get() + HOTKEY_CELLS);
            }
        }
        widths
    }

    /// The width of the table part of a row: three padded cells and the two
    /// separators between them.
    const fn table(&self) -> usize {
        self.label + self.total + self.free + 6 * PAD + 2
    }

    /// The inner box's interior, which every row is padded out to.
    fn interior(&self) -> usize {
        self.table().max(self.plugin).max(Cells::of(FOOTER).get())
    }
}

/// `" 0 "` ahead of a plugin's name, plus the space that follows it.
const HOTKEY_CELLS: usize = 4;

/// Draw the dialog with `selected` under the cursor.
///
/// The rows come in from the overlay, built once when it opened: a row
/// carries its filesystem's size, and rebuilding them here would run a
/// `statvfs` per drive on every redraw.
pub(super) fn render(f: &mut Frame<'_>, _side: PanelSide, rows: &[DriveRow], selected: usize) {
    let widths = Widths::of(rows);
    let lines = body(rows, &widths, selected);

    // The outer frame adds a column on each side, and Far leaves a column of
    // ground between the two frames.
    let area = centered(f.area(), widths.interior() + 6, lines.len() + 2);
    let ground = Style::default().bg(palette::MENU_BG).fg(palette::DRIVE_FG);

    let block = super::dialog_frame(TITLE, FOOTER, ground);

    f.render_widget(Clear, area);
    f.render_widget(Paragraph::new(lines).block(block), area);
}

/// The dialog's own rectangle: its natural size, clamped to the screen and
/// centred, rather than a percentage of it — a one-drive machine should not
/// get a half-empty box.
pub(super) fn centered(screen: Rect, width: usize, height: usize) -> Rect {
    let width = u16::try_from(width).unwrap_or(u16::MAX).min(screen.width);
    let height = u16::try_from(height).unwrap_or(u16::MAX).min(screen.height);
    Rect {
        x: screen.x + (screen.width - width) / 2,
        y: screen.y + (screen.height - height) / 2,
        width,
        height,
    }
}

#[cfg(test)]
mod tests;
#[cfg(test)]
mod tests_render;
#[cfg(test)]
mod tests_rows;
