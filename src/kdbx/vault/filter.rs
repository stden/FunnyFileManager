//! Purpose: the incremental filter over the entry list — needle edits, the
//! match rebuild and the cursor clamp that comes with it.

use std::time::Instant;

use super::Vault;
use crate::casefold::CaseNeedle;
use crate::keepass::EntrySummary;

impl Vault {
    /// Append one character to the filter.
    pub fn push_filter_char(&mut self, ch: char, now: Instant) {
        if self.filter.is_empty() {
            self.filter.push(ch);
            self.refilter(now);
            return;
        }
        if self.filter.is_ascii() && ch.is_ascii() {
            self.filter.push(ch);
            let needle = CaseNeedle::new(&self.filter);
            self.narrow_matches(&needle, now);
            return;
        }
        // Contextual Unicode folding (e.g. Greek `Σ`: `"ΟΣ"` -> `"ος"`, `"ΟΣΑ"` -> `"οσα"`)
        // can rewrite an earlier lowercase character, so only narrow when `prev` stays a prefix.
        let prev = self.filter.to_lowercase();
        self.filter.push(ch);
        if self.filter.to_lowercase().starts_with(&prev) {
            let needle = CaseNeedle::new(&self.filter);
            self.narrow_matches(&needle, now);
        } else {
            self.refilter(now);
        }
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

    fn begin_filter_edit(&mut self, now: Instant) {
        self.touch(now);
        // The revealed password belongs to whichever entry was selected when
        // `Ctrl+R` was pressed. A filter edit can move the cursor onto a
        // different entry, so keeping it would print entry A's password next to
        // entry B's name.
        self.hide();
        self.close_dialog();
    }

    /// Keep `selected` within `matches` (`0` when `matches` is empty, where
    /// `selected_entry()` returns `None`).
    fn clamp_selected_to_matches(&mut self) {
        self.selected = self.selected.min(self.matches.len().saturating_sub(1));
    }

    /// Rebuild [`Self::matches`] and re-clamp the cursor.
    pub(super) fn refilter(&mut self, now: Instant) {
        self.begin_filter_edit(now);
        self.matches.clear();
        if self.filter.is_empty() {
            self.matches.extend(0..self.entries.len());
        } else {
            let needle = CaseNeedle::new(&self.filter);
            self.matches.extend(
                self.entries
                    .iter()
                    .enumerate()
                    .filter(|(_, entry)| matches_filter(entry, &needle))
                    .map(|(index, _)| index),
            );
        }
        self.clamp_selected_to_matches();
        self.invalidate_tree_layout();
    }

    fn narrow_matches(&mut self, needle: &CaseNeedle, now: Instant) {
        self.begin_filter_edit(now);
        let entries = &self.entries;
        self.matches.retain(|&index| {
            entries
                .get(index)
                .is_some_and(|entry| matches_filter(entry, needle))
        });
        self.clamp_selected_to_matches();
        self.invalidate_tree_layout();
    }
}

/// Case-insensitive substring test over the three secret-free fields.
fn matches_filter(entry: &EntrySummary, needle: &CaseNeedle) -> bool {
    needle.contains_in(&entry.title)
        || needle.contains_in(&entry.username)
        || needle.contains_in(&entry.url)
        || entry
            .group_path
            .iter()
            .any(|group| needle.contains_in(group))
}
