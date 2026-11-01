//! Purpose: The lines of the Change drive dialog's inner box: the edges that
//! close the column separators, disk rows in three padded columns, and the
//! plugin rows under a rule, the chosen row lit to the edge.

use ratatui::{
    style::Style,
    text::{Line, Span},
};

use super::super::panel::{pad_cells, right_cells};
use super::super::{palette, Cells};
use super::{columns, Columns, Widths, HOTKEY_CELLS, PAD};
use crate::models::{DriveRow, RowKind};

/// The lines inside the outer frame: the single-line box Far draws around the
/// rows, disks in columns and plugins under a rule.
pub(super) fn body<'a>(rows: &'a [DriveRow], widths: &Widths, selected: usize) -> Vec<Line<'a>> {
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
            palette::hotkey_style(Style::default().bg(background)),
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
pub(super) fn cell(text: &str) -> String {
    format!(" {text} ")
}

fn styled(text: String) -> Line<'static> {
    Line::from(Span::styled(
        text,
        Style::default().fg(palette::DRIVE_FG).bg(palette::MENU_BG),
    ))
}
