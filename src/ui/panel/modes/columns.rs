//! Detail columns retained from menu/viewmodes, fitted to the current panel
//! renderer and its cached metadata. Narrow panels give filenames priority.

mod column;

use crate::models::{DirEntry, Panel, PanelView};

use super::super::{center_cells, pad_cells, pad_clipped_name, Cells};
use column::{columns, Column};

const MIN_NAME_WIDTH: Cells = Cells::new(8);

struct Placed {
    column: Column,
    width: Cells,
}

pub(super) struct Layout {
    name: Cells,
    columns: Vec<Placed>,
}

impl Layout {
    pub(super) fn new(view: PanelView, width: Cells) -> Self {
        let mut active = columns(view);
        while let Some((_, shorter)) = active.split_last() {
            let layout = Self::fit(view, active, width);
            if layout.name >= MIN_NAME_WIDTH {
                return layout;
            }
            active = shorter;
        }
        Self::fit(view, &[], width)
    }

    fn fit(view: PanelView, columns: &[Column], width: Cells) -> Self {
        let fixed: Cells = columns
            .iter()
            .map(|column| column.width().unwrap_or(Cells::ZERO) + Cells::new(1))
            .sum();
        let remaining = width.saturating_sub(fixed);
        let name = if columns.iter().copied().any(Column::is_flexible) {
            let divisor = if view == PanelView::LongDescriptions {
                3
            } else {
                2
            };
            Cells::new(remaining.get().div_ceil(divisor))
        } else {
            remaining
        };
        let columns = columns
            .iter()
            .map(|&column| Placed {
                column,
                width: column
                    .width()
                    .unwrap_or_else(|| remaining.saturating_sub(name)),
            })
            .collect();
        Self { name, columns }
    }

    pub(super) fn header(&self) -> String {
        self.line(center_cells("Name", self.name), |placed| {
            center_cells(placed.column.title(), placed.width)
        })
    }

    pub(super) fn row(&self, panel: &Panel, entry: &DirEntry, name: &str) -> String {
        self.line(pad_clipped_name(name, self.name), |placed| {
            let cell = placed.column.cell(panel, entry);
            placed.column.align(&cell, placed.width)
        })
    }

    /// A row below the last entry: nothing but the column lines, which run on
    /// to the divider as Full's and Medium's do.
    pub(super) fn blank(&self) -> String {
        self.line(pad_cells("", self.name), |placed| {
            pad_cells("", placed.width)
        })
    }

    /// `name`, then each column's `cell` behind its line.
    fn line(&self, name: String, cell: impl Fn(&Placed) -> String) -> String {
        let extra: usize = self
            .columns
            .iter()
            .map(|p| '│'.len_utf8() + p.width.get())
            .sum();
        let mut text = name;
        text.reserve(extra);
        for placed in &self.columns {
            text.push('│');
            text.push_str(&cell(placed));
        }
        text
    }
}
