//! Purpose: quick search and select-by-name over the panel's entries.

use super::search_index;
use super::Panel;
use crate::models::EntryKind;

impl Panel {
    /// Find and select the next entry matching the given prefix (case-insensitive).
    /// If already on a matching entry, moves to the next matching one.
    /// Returns true if a match was found.
    pub fn find_next_matching(&mut self, prefix: &str) -> bool {
        search_index::select_prefixed(self, prefix, true)
    }

    /// Put the cursor on the entry with exactly this name.
    ///
    /// Reports whether it found one, so a caller that has just created
    /// something can tell "there it is" from "the listing does not show it" —
    /// which happens when the name went somewhere the panel is not looking.
    pub fn select_named(&mut self, name: &str) -> bool {
        let Some(index) = self.entries.iter().position(|entry| entry.name == name) else {
            return false;
        };
        self.select(Some(index));
        true
    }

    /// Find and select the first entry matching the given prefix (case-insensitive).
    /// Returns true if a match was found.
    pub fn find_first_matching(&mut self, prefix: &str) -> bool {
        search_index::select_prefixed(self, prefix, false)
    }

    /// What the entry at `path` is, as the listing read it — for a question
    /// asked about it, F8's "delete the folder?". `None` for a path the
    /// listing does not hold.
    #[must_use]
    pub fn kind_of(&self, path: &std::path::Path) -> Option<EntryKind> {
        self.entries
            .iter()
            .find(|entry| self.path.join(entry.disk_name()) == path)
            .map(|entry| entry.kind)
    }
}
