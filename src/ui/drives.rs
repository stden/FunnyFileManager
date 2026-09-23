//! Purpose: Far's Alt+F1 / Alt+F2 "Change drive" dialog.
//!
//! Far draws it as a menu with columns, not as a form: a boxed table of the
//! machine's filesystems — letter, kind, size, free — and under a rule
//! whatever else can be opened as a panel. The inner frame is drawn from
//! characters here rather than nested `Block`s, because the rule between the
//! two sections has to close the column separators above it (`┴`), which a
//! border cannot express.

use ratatui::{
    layout::Rect,
    style::Style,
    text::{Line, Span},
    widgets::{Clear, Paragraph},
    Frame,
};

use super::panel::{abbreviate_bytes, pad_cells, right_cells};
use super::{palette, Cells};
use crate::models::{DriveRow, PanelSide, RowKind};

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
pub(super) fn render(f: &mut Frame, _side: PanelSide, rows: &[DriveRow], selected: usize) {
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

/// The lines inside the outer frame: the single-line box Far draws around the
/// rows, disks in columns and plugins under a rule.
fn body<'a>(rows: &'a [DriveRow], widths: &Widths, selected: usize) -> Vec<Line<'a>> {
    let mut lines = Vec::with_capacity(rows.len() + 3);
    let mut ruled = false;

    lines.push(edge('┌', '┬', '┐', widths));
    for (index, row) in rows.iter().enumerate() {
        if !row.is_disk() && !ruled {
            // The rule closes the separators above it and opens nothing below.
            lines.push(edge('├', '┴', '┤', widths));
            ruled = true;
        }
        lines.push(row_line(row, widths, index == selected));
    }
    // Below the last disk the separators still have to be closed; below a
    // plugin section there is nothing to close.
    lines.push(if ruled {
        plain_edge('└', '┘', widths)
    } else {
        edge('└', '┴', '┘', widths)
    });
    lines
}

/// A horizontal edge that closes the column separators with `joint`.
fn edge(left: char, joint: char, right: char, widths: &Widths) -> Line<'static> {
    if widths.disks == 0 {
        return plain_edge(left, right, widths);
    }
    let mut text = String::from(left);
    for width in [widths.label, widths.total, widths.free] {
        text.push_str(&"─".repeat(width + 2 * PAD));
        text.push(joint);
    }
    // The last joint was one too many; the row may also be narrower than the
    // interior, which the plugin rows or the footer widened.
    text.pop();
    text.push_str(&"─".repeat(widths.interior().saturating_sub(widths.table())));
    text.push(right);
    styled(text)
}

/// A horizontal edge with nothing to close.
fn plain_edge(left: char, right: char, widths: &Widths) -> Line<'static> {
    styled(format!("{left}{}{right}", "─".repeat(widths.interior())))
}

/// One row: three columns for a disk, a hotkey and a name for anything else.
fn row_line<'a>(row: &'a DriveRow, widths: &Widths, chosen: bool) -> Line<'a> {
    let (name, number, frame) = row_styles(chosen);
    let background = number.bg.unwrap_or(palette::MENU_BG);
    let mut spans = vec![Span::styled("│", frame)];
    if let Some(cols) = columns(&row.kind) {
        append_disk_columns(&mut spans, &cols, widths, name, number, frame);
    } else if let RowKind::Plugin { name: label } = &row.kind {
        // A keyless row still pays for the hotkey cell, or its name would sit
        // one column left of the keyed rows above it.
        let hotkey = row
            .key
            .map_or_else(|| "   ".to_owned(), |k| format!(" {k} "));
        spans.push(Span::styled(
            hotkey,
            Style::default().fg(palette::HOTKEY).bg(background),
        ));
        spans.push(Span::styled(
            pad_cells(
                label,
                Cells::new(widths.interior().saturating_sub(HOTKEY_CELLS - 1)),
            ),
            name,
        ));
    }
    spans.push(Span::styled("│", frame));
    Line::from(spans)
}

fn row_styles(chosen: bool) -> (Style, Style, Style) {
    let background = if chosen {
        palette::DRIVE_SELECTED_BG
    } else {
        palette::MENU_BG
    };
    // Far marks the row under the cursor by inverting it and turning the name
    // itself the hotkey yellow; the sizes stay white so the numbers still read
    // as numbers.
    let name = Style::default()
        .fg(if chosen {
            palette::HOTKEY
        } else {
            palette::DRIVE_FG
        })
        .bg(background);
    let number = Style::default().fg(palette::DRIVE_FG).bg(background);
    let frame = Style::default().fg(palette::DRIVE_FG).bg(palette::MENU_BG);

    (name, number, frame)
}

fn append_disk_columns(
    spans: &mut Vec<Span<'static>>,
    cols: &Columns,
    widths: &Widths,
    name: Style,
    number: Style,
    frame: Style,
) {
    spans.push(Span::styled(
        cell(&pad_cells(&cols.label, Cells::new(widths.label))),
        name,
    ));
    spans.push(Span::styled("│", frame));
    spans.push(Span::styled(
        cell(&right_cells(&cols.total, Cells::new(widths.total))),
        number,
    ));
    spans.push(Span::styled("│", frame));
    spans.push(Span::styled(
        cell(&right_cells(&cols.free, Cells::new(widths.free))),
        number,
    ));
    // The filler out to the interior belongs to the row, not the frame:
    // painted with the frame's ground it cut the selection bar short
    // whenever a plugin row or the footer made the box wider than the
    // table.
    spans.push(Span::styled(
        " ".repeat(widths.interior().saturating_sub(widths.table())),
        number,
    ));
}

/// A column's text with Far's single space either side.
///
/// Alignment inside a column is the panel's [`pad_cells`] and [`right_cells`]:
/// `right_cells` can truncate, but every width here is the maximum
/// [`Widths::of`] measured over these same rows, so nothing arrives wider.
fn cell(text: &str) -> String {
    format!(" {text} ")
}

fn styled(text: String) -> Line<'static> {
    Line::from(Span::styled(
        text,
        Style::default().fg(palette::DRIVE_FG).bg(palette::MENU_BG),
    ))
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
