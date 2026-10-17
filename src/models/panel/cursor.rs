//! Purpose: moves and validates the panel's highlighted row.

use ratatui::widgets::ListState;
use std::path::PathBuf;

use crate::models::{DirEntry, PanelKind};

use super::Panel;

impl Panel {
    /// Move down one entry, stopping at the last.
    ///
    /// No wrap. Far does not wrap, and holding Down to reach the bottom of a
    /// long listing should end at the bottom — wrapping puts the cursor back on
    /// `..` and one more keypress leaves the directory, which is the opposite
    /// of what the user was doing.
    pub fn next(&mut self) {
        if self.entries.is_empty() {
            return;
        }
        let last = self.entries.len() - 1;
        let index = self.state.selected().map_or(0, |at| (at + 1).min(last));
        self.select(Some(index));
    }

    /// Move up one entry, stopping at the first — the mirror of [`Self::next`].
    pub fn previous(&mut self) {
        if self.entries.is_empty() {
            return;
        }
        let index = self.state.selected().map_or(0, |at| at.saturating_sub(1));
        self.select(Some(index));
    }

    /// Move the selection down one screen, stopping on the last entry.
    pub fn page_down(&mut self, page: usize) {
        self.page_by(|i, last| i.saturating_add(page).min(last));
    }

    /// Move the selection up one screen, stopping on the first entry.
    pub fn page_up(&mut self, page: usize) {
        self.page_by(|i, _| i.saturating_sub(page));
    }

    /// Shared paging core.
    ///
    /// Paging clamps at both ends rather than wrapping like [`Self::next`] /
    /// [`Self::previous`] — Far Manager parks on the last entry when `PgDn` runs
    /// past the end. With nothing selected we select the first entry, which is
    /// what `next` / `previous` do.
    fn page_by(&mut self, step: impl Fn(usize, usize) -> usize) {
        if self.entries.is_empty() {
            return;
        }
        let last = self.entries.len() - 1;
        let target = match self.state.selected() {
            Some(i) => step(i, last),
            None => 0,
        };
        self.select(Some(target));
    }

    #[must_use]
    pub fn get_selected_entry(&self) -> Option<&DirEntry> {
        self.state.selected().and_then(|i| self.entries.get(i))
    }

    /// The highlighted row, if any.
    #[must_use]
    pub fn selected(&self) -> Option<usize> {
        self.state.selected()
    }

    /// The row the cursor is on in what the panel shows: a `NetBox` list's
    /// own cursor, else the file list's highlight. The status line gives the
    /// info row back once this moves.
    #[must_use]
    pub fn shown_cursor(&self) -> Option<usize> {
        match &self.kind {
            PanelKind::Sessions(sessions) => Some(sessions.cursor()),
            PanelKind::Remote(remote) => Some(remote.cursor()),
            PanelKind::Files | PanelKind::Tree(_) | PanelKind::QuickView(_) => self.selected(),
        }
    }

    /// Highlight `index`, or nothing.
    ///
    /// An index past the end becomes `None`: it names no row, so highlighting
    /// it would render nothing while `get_selected_entry` reported "no file
    /// selected" — a bug wearing an ordinary state's clothes.
    pub fn select(&mut self, index: Option<usize>) {
        let existing = index.filter(|i| *i < self.entries.len());
        self.state.select(existing);
    }

    /// The list widget's state, for rendering only.
    pub const fn list_state_mut(&mut self) -> &mut ListState {
        &mut self.state
    }

    pub fn ensure_selection(&mut self) {
        if !self.entries.is_empty() && self.state.selected().is_none() {
            self.select(Some(0));
        }
    }

    #[must_use]
    pub fn selected_path(&self) -> Option<PathBuf> {
        self.get_selected_entry().and_then(|entry| {
            if entry.name == ".." {
                None
            } else {
                Some(self.path.join(entry.disk_name()))
            }
        })
    }
}
