//! Purpose: read-only accessors over an unlocked vault, for the renderer.

use std::time::Instant;

use super::Vault;
use crate::kdbx::edit::EntryForm;
use crate::kdbx::secret::Revealed;
use crate::keepass::EntrySummary;

impl Vault {
    // ── Read-only accessors for the renderer ────────────────────────────

    pub fn entries(&self) -> &[EntrySummary] {
        &self.entries
    }

    /// Indices into [`Self::entries`] that currently pass the filter.
    pub fn matches(&self) -> &[usize] {
        &self.matches
    }

    /// The entries to draw, in display order.
    pub fn visible(&self) -> impl Iterator<Item = &EntrySummary> {
        self.matches
            .iter()
            .filter_map(|&index| self.entries.get(index))
    }

    pub fn filter(&self) -> &str {
        &self.filter
    }

    /// Cursor position **within [`Self::matches`]**, always `< matches.len()`
    /// whenever any entry matches.
    pub fn selected(&self) -> usize {
        self.selected
    }

    pub fn selected_entry(&self) -> Option<&EntrySummary> {
        self.entries.get(*self.matches.get(self.selected)?)
    }

    /// The revealed password, or `None` while it is masked.
    pub fn revealed(&self) -> Option<&str> {
        self.revealed.as_ref().map(Revealed::expose)
    }

    pub fn is_revealed(&self) -> bool {
        self.revealed.is_some()
    }

    /// The open edit form, or `None` while the user is browsing.
    pub fn form(&self) -> Option<&EntryForm> {
        self.edit.as_deref()
    }

    pub fn form_mut(&mut self) -> Option<&mut EntryForm> {
        self.edit.as_deref_mut()
    }

    pub fn last_activity(&self) -> Instant {
        self.last_activity
    }

    pub(crate) fn groups(&self) -> &[Vec<String>] {
        &self.groups
    }
}
