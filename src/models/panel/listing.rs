//! Purpose: Committing a freshly read listing to the panel: filtering and
//! sorting it, trimming stale marks, recording the move, and putting the
//! cursor back where it belongs.

use std::{
    collections::{HashMap, HashSet},
    path::PathBuf,
};

use super::sort::sort_entries;
use super::{Cursor, DirectoryListing, Panel};

impl Panel {
    /// Commit a complete listing while preserving selection and valid marks.
    pub(crate) fn apply_listing(&mut self, mut listing: DirectoryListing) {
        let previous = self.cursor();
        // Two filters narrow a listing and both must run: the panel's own mask
        // from Ctrl+I, and the File panel filter the menu sets, which also
        // hides dotfiles when the panel is asked to.
        self.filter.apply(&mut listing.entries);
        self.filter_listing(&mut listing);
        sort_entries(&mut listing.entries, self.sort_mode, self.sort_order);
        let mut entries = Vec::with_capacity(listing.entries.len() + 1);
        entries.extend(listing.parent);
        entries.append(&mut listing.entries);
        self.entries = entries;
        self.metadata = listing.metadata;
        *self.short_names.borrow_mut() = None;
        self.forget_unlisted_marks();
        self.select_after_reload(&previous);
    }

    /// Drop every selected or remembered name the listing no longer shows.
    ///
    /// Both sets hold bare names that F5 and F8 join onto `path`, so a name
    /// that is not listed here names a file that is gone — or a namesake.
    /// The remembered set is trimmed too: Ctrl+M swaps it back in, and a
    /// deleted file must not come back as a mark that F8 then offers again.
    fn forget_unlisted_marks(&mut self) {
        if self.marked.is_empty() && self.previous_marks.is_empty() {
            return;
        }
        let listed: HashSet<&str> = self
            .entries
            .iter()
            .filter(|entry| entry.name != "..")
            .map(|entry| entry.name.as_str())
            .collect();
        self.marked.retain(|name| listed.contains(name.as_str()));
        self.previous_marks
            .retain(|name| listed.contains(name.as_str()));
    }

    /// Commit navigation only after the target directory has been loaded.
    pub(crate) fn apply_navigation(&mut self, path: PathBuf, listing: DirectoryListing) {
        self.kind = super::PanelKind::Files;
        self.pending_visit = Some(path.clone());
        if self.navigation_history.back() != Some(&self.path) {
            self.navigation_history.push_back(self.path.clone());
        }
        if self.navigation_history.len() > 100 {
            self.navigation_history.pop_front();
        }
        self.path = path;
        // Marks are names in the folder being left. A namesake in the new one
        // is a different file, so neither set may follow the panel there.
        self.marked.clear();
        self.previous_marks.clear();
        self.apply_listing(listing);
        self.ensure_selection();
    }

    /// Where the cursor is now, for restoring it after a reload.
    pub(crate) fn cursor(&self) -> Cursor {
        Cursor {
            name: self.get_selected_entry().map(|entry| entry.name.clone()),
            index: self.state.selected().unwrap_or(0),
        }
    }

    pub(crate) fn select_after_reload(&mut self, previous: &Cursor) {
        let Some(name) = previous.name.as_deref() else {
            // Nothing was selected, so there is nothing to restore.
            if self.entries.is_empty() {
                self.select(None);
            }
            return;
        };
        if self.entries.is_empty() {
            self.select(None);
            return;
        }
        if let Some(index) = self.entries.iter().position(|entry| entry.name == name) {
            self.select(Some(index));
            return;
        }
        // The entry is gone — deleted, renamed, or moved away. Far leaves the
        // cursor where it was, so it lands on whatever took the entry's place;
        // jumping back to the top loses the user's position in a long listing
        // exactly when they are working through it one file at a time.
        let last = self.entries.len() - 1;
        self.select(Some(previous.index.min(last)));
    }

    #[must_use]
    pub(crate) fn selected_is_parent_dir(&self) -> bool {
        self.get_selected_entry()
            .is_some_and(|entry| entry.name == "..")
    }

    #[must_use]
    pub(crate) fn directory_targets(&self) -> Vec<(String, PathBuf)> {
        self.entries
            .iter()
            .filter(|entry| entry.kind.is_dir() && entry.name != "..")
            .map(|entry| (entry.name.clone(), self.path.join(entry.disk_name())))
            .collect()
    }

    pub(crate) fn set_directory_size_at(
        &mut self,
        target: &std::path::Path,
        size: crate::models::FileSize,
    ) {
        let base = self.path.clone();
        if let Some(entry) = self.entries.iter_mut().find(|entry| {
            entry.kind.is_dir() && entry.name != ".." && base.join(entry.disk_name()) == target
        }) {
            entry.set_measured_size(size);
        }
        self.resort_if_by_size();
    }

    pub(crate) fn set_directory_sizes(&mut self, measured: &[(String, crate::models::FileSize)]) {
        let by_name: HashMap<&str, crate::models::FileSize> = measured
            .iter()
            .map(|(name, size)| (name.as_str(), *size))
            .collect();
        for entry in &mut self.entries {
            if let Some(&size) = by_name
                .get(entry.name.as_str())
                .filter(|_| entry.kind.is_dir())
            {
                entry.set_measured_size(size);
            }
        }
        self.resort_if_by_size();
    }

    fn resort_if_by_size(&mut self) {
        if self.sort_mode != crate::models::SortMode::Size {
            return;
        }
        let previous = self.cursor();
        let start = usize::from(self.entries.first().is_some_and(|entry| entry.name == ".."));
        if let Some(rest) = self.entries.get_mut(start..) {
            sort_entries(rest, self.sort_mode, self.sort_order);
        }
        *self.short_names.borrow_mut() = None;
        self.select_after_reload(&previous);
    }
}
