//! Purpose: The state of the Things catalog dialog (F9 Commands > Things
//! catalog): what it waits for, the machines it shows, and the filter, cursor
//! and marks set on them.

use std::{collections::BTreeSet, sync::Arc};

use crate::models::{RowCursor, RowMotion};
use crate::things::{self, Config, Listing, Machine};

/// Whether this program may read the catalog. Only live input may: a test or a
/// replayed session reads nothing.
#[derive(Clone, Copy, Default)]
pub enum ThingsAccess {
    #[default]
    Offline,
    Network,
}

/// Where the dialog is.
#[derive(Clone)]
pub enum ThingsState {
    /// The settings are missing or unusable; the text says which.
    Setup(String),
    /// The settings are good; the read starts on the next turn of the loop.
    Queued(Config),
    /// The read is out.
    Loading,
    /// The machines, with the filter, cursor and marks set on them.
    Ready(ThingsCatalog),
    /// The read failed; the text says why.
    Failed(String),
}

/// The Things catalog dialog. `generation` tells the reply to a read apart from
/// the reply to a dialog that has since been closed or reopened.
#[derive(Clone)]
pub struct ThingsDialog {
    pub(crate) generation: Arc<()>,
    pub(crate) state: ThingsState,
}

impl ThingsDialog {
    #[must_use]
    pub fn new(state: ThingsState) -> Self {
        Self {
            generation: Arc::new(()),
            state,
        }
    }

    /// The settings of a read that is waiting, which marks the dialog as
    /// loading; `None` when no read is waiting.
    pub fn start(&mut self) -> Option<Config> {
        match std::mem::replace(&mut self.state, ThingsState::Loading) {
            ThingsState::Queued(config) => Some(config),
            other => {
                self.state = other;
                None
            }
        }
    }
}

/// The machines of one read, the filter typed so far, the cursor, and the
/// machines marked for import.
#[derive(Clone)]
pub struct ThingsCatalog {
    machines: Vec<Machine>,
    total: Option<usize>,
    filter: String,
    cursor: RowCursor,
    /// Indices into `machines`, so a mark survives the filter changing.
    marked: BTreeSet<usize>,
}

impl ThingsCatalog {
    #[must_use]
    pub fn new(listing: Listing) -> Self {
        Self {
            machines: listing.machines,
            total: listing.total,
            filter: String::new(),
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
        self.visible()
            .into_iter()
            .filter_map(|index| {
                self.machines
                    .get(index)
                    .map(|machine| (self.marked.contains(&index), machine))
            })
            .collect()
    }

    pub fn move_cursor(&mut self, motion: RowMotion) {
        self.cursor = self.cursor.moved(motion, self.visible().len());
    }

    /// Mark the row under the cursor, or unmark it.
    pub fn toggle_mark(&mut self) {
        let Some(&index) = self.visible().get(self.cursor.index()) else {
            return;
        };
        if !self.marked.remove(&index) {
            self.marked.insert(index);
        }
    }

    /// Append `ch` to the filter. The list starts again at its top.
    pub fn type_filter(&mut self, ch: char) {
        self.filter.push(ch);
        self.cursor = RowCursor::TOP;
    }

    /// Remove the filter's last character. The list starts again at its top.
    pub fn erase_filter(&mut self) {
        if self.filter.pop().is_some() {
            self.cursor = RowCursor::TOP;
        }
    }

    /// What an import takes: the marked machines when any are marked, or else
    /// every machine the filter shows.
    #[must_use]
    pub fn chosen(&self) -> Vec<&Machine> {
        let indices = if self.marked.is_empty() {
            self.visible()
        } else {
            self.marked.iter().copied().collect()
        };
        indices
            .into_iter()
            .filter_map(|index| self.machines.get(index))
            .collect()
    }

    fn visible(&self) -> Vec<usize> {
        self.machines
            .iter()
            .enumerate()
            .filter(|(_, machine)| things::matches(machine, &self.filter))
            .map(|(index, _)| index)
            .collect()
    }
}

#[cfg(test)]
mod tests;
