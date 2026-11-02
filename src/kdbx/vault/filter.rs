//! Purpose: the incremental filter over the entry list — needle edits, the
//! match rebuild and the cursor clamp that comes with it.

use std::time::Instant;

use super::Vault;
use crate::casefold::contains_ignore_case;
use crate::keepass::EntrySummary;

impl Vault {
    /// Append one character to the filter.
    pub fn push_filter_char(&mut self, ch: char, now: Instant) {
        self.filter.push(ch);
        self.refilter(now);
    }

    /// Delete the last character of the filter.
    pub fn pop_filter_char(&mut self, now: Instant) {
        self.filter.pop();
        self.refilter(now);
    }

    /// Replace the filter outright.
    pub fn set_filter(&mut self, filter: &str, now: Instant) {
        self.filter.clear();
        self.filter.push_str(filter);
        self.refilter(now);
    }

    // ── Internals ───────────────────────────────────────────────────────

    /// Rebuild [`Self::matches`] and re-clamp the cursor.
    pub(super) fn refilter(&mut self, now: Instant) {
        self.touch(now);
        // The revealed password belongs to whichever entry was selected when
        // `Ctrl+R` was pressed. A filter edit can move the cursor onto a
        // different entry, so keeping it would print entry A's password next to
        // entry B's name.
        self.hide();
        self.close_dialog();

        let needle = self.filter.to_lowercase();
        self.matches = self
            .entries
            .iter()
            .enumerate()
            .filter(|(_, entry)| matches_filter(entry, &needle))
            .map(|(index, _)| index)
            .collect();

        // `selected` indexes `matches`, and narrowing the filter can shrink
        // `matches` out from under it. An index that outlives the list it points
        // into has already shipped once in this codebase (panel reload), so it
        // is clamped here — the single place `matches` is rebuilt — instead of
        // at every read site. With no matches this parks at 0 and
        // `selected_entry()` correctly returns `None`.
        self.selected = self.selected.min(self.matches.len().saturating_sub(1));
    }
}

/// Case-insensitive substring test over the three secret-free fields.
///
/// `needle` must already be lowercased — [`Vault::refilter`] does it once per
/// edit rather than once per entry.
fn matches_filter(entry: &EntrySummary, needle: &str) -> bool {
    [&entry.title, &entry.username, &entry.url]
        .into_iter()
        .any(|field| contains_ignore_case(field, needle))
        || entry
            .group_path
            .iter()
            .any(|group| contains_ignore_case(group, needle))
}
