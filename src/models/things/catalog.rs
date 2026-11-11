//! Purpose: Filtered, markable machine catalog inside the Things dialog.

use std::collections::BTreeSet;

use crate::casefold::CaseNeedle;
use crate::models::{RowCursor, RowMotion};
use crate::things::{self, Listing, Machine};

/// The machines of one read, the filter typed so far, the cursor, and the
/// machines marked for import.
#[derive(Clone)]
pub struct ThingsCatalog {
    machines: Vec<Machine>,
    total: Option<usize>,
    filter: String,
    visible: Vec<usize>,
    cursor: RowCursor,
    /// Indices into `machines`, so a mark survives the filter changing.
    marked: BTreeSet<usize>,
}

impl ThingsCatalog {
    #[must_use]
    pub fn new(listing: Listing) -> Self {
        let visible = (0..listing.machines.len()).collect();
        Self {
            machines: listing.machines,
            total: listing.total,
            filter: String::new(),
            visible,
            cursor: RowCursor::TOP,
            marked: BTreeSet::new(),
        }
    }

    /// The server's total, when it is larger than the machines read.
    #[must_use]
    pub fn unshown_total(&self) -> Option<usize> {
        self.total.filter(|total| *total > self.machines.len())
    }

    #[must_use]
    pub fn machine_count(&self) -> usize {
        self.machines.len()
    }

    #[must_use]
    pub fn filter(&self) -> &str {
        &self.filter
    }

    /// The cursor's place among the rows the filter shows.
    #[must_use]
    pub fn cursor_index(&self) -> usize {
        self.cursor.index()
    }

    /// The rows the filter shows, each with whether it is marked.
    #[must_use]
    pub fn shown(&self) -> Vec<(bool, &Machine)> {
        self.visible
            .iter()
            .filter_map(|&index| {
                self.machines
                    .get(index)
                    .map(|machine| (self.marked.contains(&index), machine))
            })
            .collect()
    }

    pub fn move_cursor(&mut self, motion: RowMotion) {
        self.cursor = self.cursor.moved(motion, self.visible.len());
    }

    /// Mark the row under the cursor, or unmark it.
    pub fn toggle_mark(&mut self) {
        let Some(&index) = self.visible.get(self.cursor.index()) else {
            return;
        };
        if !self.marked.remove(&index) {
            self.marked.insert(index);
        }
    }

    /// Append `ch` to the filter. The list starts again at its top.
    pub fn type_filter(&mut self, ch: char) {
        let can_narrow = !self.filter.is_empty() && self.filter.is_ascii() && ch.is_ascii();
        self.filter.push(ch);
        self.cursor = RowCursor::TOP;
        if can_narrow {
            let needle = CaseNeedle::new(&self.filter);
            let machines = &self.machines;
            self.visible.retain(|&idx| {
                machines
                    .get(idx)
                    .is_some_and(|machine| things::matches_needle(machine, &needle))
            });
        } else {
            self.rebuild_visible();
        }
    }

    /// Remove the filter's last character. The list starts again at its top.
    pub fn erase_filter(&mut self) {
        if self.filter.pop().is_some() {
            self.cursor = RowCursor::TOP;
            self.rebuild_visible();
        }
    }

    /// What an import takes: the marked machines when any are marked, or else
    /// every machine the filter shows.
    #[must_use]
    pub fn chosen(&self) -> Vec<&Machine> {
        if self.marked.is_empty() {
            self.visible
                .iter()
                .filter_map(|&index| self.machines.get(index))
                .collect()
        } else {
            self.marked
                .iter()
                .filter_map(|&index| self.machines.get(index))
                .collect()
        }
    }

    fn rebuild_visible(&mut self) {
        self.visible.clear();
        if self.filter.is_empty() {
            self.visible.extend(0..self.machines.len());
            return;
        }
        let needle = CaseNeedle::new(&self.filter);
        self.visible.extend(
            self.machines
                .iter()
                .enumerate()
                .filter(|(_, machine)| things::matches_needle(machine, &needle))
                .map(|(index, _)| index),
        );
    }
}
