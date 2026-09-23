//! Purpose: quick search and select-by-name over the panel's entries.

use crate::casefold::starts_with_ignore_case;

use super::Panel;

impl Panel {
    /// Find and select the next entry matching the given prefix (case-insensitive).
    /// If already on a matching entry, moves to the next matching one.
    /// Returns true if a match was found.
    pub fn find_next_matching(&mut self, prefix: &str) -> bool {
        if self.entries.is_empty() || prefix.is_empty() {
            return false;
        }

        let current = self.state.selected().unwrap_or(0);
        let len = self.entries.len();

        // Search from current+1 to end, then from 0 to current (inclusive)
        for offset in 1..=len {
            let idx = (current + offset) % len;
            let is_match = self.entries.get(idx).is_some_and(|entry| {
                entry.name != ".." && starts_with_ignore_case(&entry.name, prefix)
            });
            if is_match {
                self.select(Some(idx));
                return true;
            }
        }

        false
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
        if self.entries.is_empty() || prefix.is_empty() {
            return false;
        }

        for (idx, entry) in self.entries.iter().enumerate() {
            if entry.name != ".." && starts_with_ignore_case(&entry.name, prefix) {
                self.select(Some(idx));
                return true;
            }
        }

        false
    }
}
