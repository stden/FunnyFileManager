//! Purpose: The task list's table — which columns fit, their titles, one
//! line per shown process, and the rules above and below — laid out in cells
//! by one set of widths that the renderer and the title clicks share.
//!
//! Columns give way in a fixed order until the Command column keeps
//! [`MIN_COMMAND`] cells; PID, CPU% and Command never do. Every text a
//! process supplies passes through `printable_name` before it is measured.

mod cells;
mod column;

use ratatui::style::Style;
use ratatui::text::{Line, Span};

use crate::models::{SortDirection, TaskList, ViewRow};

use super::super::panel::{center_cells, cursor_style, separator_style, Mark, COLUMN_SEPARATOR};
use super::super::{palette, Cells};
use cells::{cell, Tone};
pub(super) use column::Column;

/// What a column separator costs.
const SEPARATOR: Cells = Cells::new(1);
/// The Command column's floor: columns are dropped until it has this many.
const MIN_COMMAND: Cells = Cells::new(16);
/// The PID column is at least this wide, and wider for longer PIDs.
const MIN_PID: Cells = Cells::new(5);

/// Whether a row is drawn under the cursor bar.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum RowBar {
    Cursor,
    Plain,
}

/// The columns that fit a table, each with its width.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct Columns(Vec<(Column, Cells)>);

impl Columns {
    /// The columns for a table `interior` cells wide whose PIDs are up to
    /// `pid_digits` long.
    pub(super) fn fit(interior: Cells, pid_digits: usize) -> Self {
        let pid = MIN_PID.max(Cells::new(pid_digits));
        let mut shown = Column::ALL.to_vec();
        for dropped in Column::DROP_ORDER {
            if command_room(&shown, interior, pid) >= MIN_COMMAND {
                break;
            }
            shown.retain(|column| *column != dropped);
        }
        let command = command_room(&shown, interior, pid);
        let widths = shown.iter().map(|&column| match column {
            Column::Command => (column, command),
            _ => (column, column.fixed_width(pid)),
        });
        Self(widths.collect())
    }

    /// The columns for `list` on a table `interior` cells wide.
    pub(super) fn for_list(list: &TaskList, interior: Cells) -> Self {
        let largest = list.rows().iter().map(|row| row.sample().key.pid().get());
        let digits = largest.max().map_or(0, |pid| pid.to_string().len());
        Self::fit(interior, digits)
    }

    /// The column drawn at `x` cells from the table's left edge; `None` on
    /// a separator or past the last column.
    pub(super) fn at(&self, x: usize) -> Option<Column> {
        let mut left = 0_usize;
        for &(column, width) in &self.0 {
            let right = left.saturating_add(width.get());
            if x < right {
                return (x >= left).then_some(column);
            }
            left = right.saturating_add(SEPARATOR.get());
        }
        None
    }

    /// A rule across the frame, `left` and `right` joining it and `junction`
    /// under each separator: `╟──┬──╢` or `╟──┴──╢`.
    pub(super) fn rule(&self, left: char, junction: char, right: char) -> String {
        let segments: Vec<String> = self
            .0
            .iter()
            .map(|(_, width)| "─".repeat(width.get()))
            .collect();
        format!("{left}{}{right}", segments.join(&junction.to_string()))
    }

    /// The column titles, the sort column's in cursor colours with its
    /// arrow where the arrow fits.
    pub(super) fn titles(&self, list: &TaskList) -> Line<'static> {
        let sort = list.options().sort;
        let view = list.command_view();
        let arrow = match sort.direction() {
            SortDirection::Ascending => "▲",
            SortDirection::Descending => "▼",
        };
        let cells = self.0.iter().map(|&(column, width)| {
            let title = column.title(view);
            if column.sort_key(view) != sort.key() {
                return Span::styled(center_cells(title, width), data());
            }
            let marked = format!("{title}{arrow}");
            let text = if Cells::of(&marked) <= width {
                marked
            } else {
                title.to_owned()
            };
            Span::styled(center_cells(&text, width), sorted_title())
        });
        Line::from(separated(cells.collect()))
    }

    /// The line for `shown`, one of `list`'s rows, lit when it is under the
    /// cursor bar.
    pub(super) fn row(&self, list: &TaskList, shown: ViewRow, bar: RowBar) -> Line<'static> {
        let Some(process) = list.rows().get(shown.row()) else {
            return self.empty();
        };
        let mark = if list.is_marked(shown.key()) {
            Mark::Marked
        } else {
            Mark::Unmarked
        };
        let tone = Tone::of(process, mark);
        let cells = self.0.iter().map(|&(column, width)| {
            let text = cell(list, process, shown.line(), column, width);
            Span::styled(text, tone.style(column, process.sample().state))
        });
        let spans = separated(cells.collect());
        match bar {
            RowBar::Cursor => {
                let style = cursor_style(mark);
                let under = spans.into_iter().map(|span| span.patch_style(style));
                Line::from(under.collect::<Vec<_>>())
            }
            RowBar::Plain => Line::from(spans),
        }
    }

    /// A line below the last process: only the column rules.
    pub(super) fn empty(&self) -> Line<'static> {
        let cells = self
            .0
            .iter()
            .map(|(_, width)| Span::styled(" ".repeat(width.get()), data()));
        Line::from(separated(cells.collect()))
    }
}

/// Cells left for Command after the `shown` columns and their separators.
fn command_room(shown: &[Column], interior: Cells, pid: Cells) -> Cells {
    let fixed: usize = shown
        .iter()
        .map(|column| column.fixed_width(pid).get())
        .sum();
    let separators = shown.len().saturating_sub(1);
    interior.saturating_sub(Cells::new(fixed.saturating_add(separators)))
}

/// `cells` with a column rule between each two.
fn separated(cells: Vec<Span<'static>>) -> Vec<Span<'static>> {
    let mut spans = Vec::with_capacity(cells.len().saturating_mul(2));
    for (index, cell) in cells.into_iter().enumerate() {
        if index > 0 {
            spans.push(Span::styled(COLUMN_SEPARATOR, separator_style()));
        }
        spans.push(cell);
    }
    spans
}

fn data() -> Style {
    Style::default().fg(palette::DATA)
}

fn sorted_title() -> Style {
    Style::default()
        .fg(palette::CURSOR_FG)
        .bg(palette::CURSOR_BG)
}
