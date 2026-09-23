//! File panel state: directory snapshots, selection, marks and quick search.
//! Filesystem loading and navigation effects live in `ops::panel`.

use ratatui::widgets::ListState;
use std::{
    collections::{BTreeSet, HashSet},
    path::PathBuf,
};

use super::{DirEntry, FileName, FileSize, PanelFilter, PanelKind, SortMode};

mod cursor;
mod marks;
mod search;
mod sort;
pub mod view;
pub use view::{PanelDetails, PanelOptions, PanelView};

use sort::sort_entries;

/// Filesystem facts captured with a directory listing.
///
/// Keeping these on the panel preserves existing public `DirEntry` literals.
/// Bare-name keys remain stable when the entries are sorted.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PanelMetadata {
    pub(crate) hidden_names: BTreeSet<FileName>,
    pub(crate) available_space: Option<FileSize>,
}

/// A fully loaded directory, published only after loading succeeds.
#[derive(Debug)]
pub struct DirectoryListing {
    pub(crate) entries: Vec<DirEntry>,
    pub(crate) parent: Option<DirEntry>,
    pub(crate) metadata: PanelMetadata,
}

/// File panel — one half of the screen (left or right)
#[derive(Debug, Clone)]
pub struct Panel {
    pub path: PathBuf,
    pub entries: Vec<DirEntry>,
    /// Private: the highlighted row and the entry list share an invariant —
    /// the index must name a row that exists. It was enforced by hand at every
    /// call site, including a written-out `filter(|i| *i < entries.len())`.
    /// [`Panel::select`] does it once instead.
    state: ListState,
    pub marked: BTreeSet<FileName>,
    /// What was marked before the last change to `marked`, so Ctrl+M can put
    /// it back. Far calls it "restore selection", and it is what saves a user
    /// who has just typed the wrong mask over a careful hand-made selection.
    /// Like `marked`, it only ever names entries of the current listing.
    previous_marks: BTreeSet<FileName>,
    pub sort_mode: SortMode,
    /// `-` on the sort menu: the chosen order, backwards.
    pub sort_reversed: bool,
    /// Which of Far's ten column layouts the panel draws.
    pub view: PanelView,
    /// A file list, a folder tree, or a preview of the other panel's cursor.
    pub kind: PanelKind,
    /// `Ctrl+I`: what the listing leaves out. Applied where the listing is
    /// committed, so every reload sees it.
    pub filter: PanelFilter,
    /// Which description files a listing consults, in order —
    /// `Settings.descriptions.names`, copied here because the panel loads
    /// itself without an `App` in reach.
    pub description_names: Vec<String>,
    /// A directory entered but not yet written to the folder history. The
    /// panel does not know the `App` that owns the history, so it leaves the
    /// note here and [`App::record_folder_visits`](super::App::record_folder_visits)
    /// collects it once per event.
    pub(crate) pending_visit: Option<PathBuf>,
    pub details: PanelDetails,
    pub options: PanelOptions,
    pub navigation_history: Vec<PathBuf>,
    /// Facts captured with the listing, never queried while rendering.
    metadata: PanelMetadata,
}

impl Panel {
    /// Commit a complete listing while preserving selection and valid marks.
    pub(crate) fn apply_listing(&mut self, mut listing: DirectoryListing) {
        let previous = self.cursor();
        // Two filters narrow a listing and both must run: the panel's own mask
        // from Ctrl+I, and the File panel filter the menu sets, which also
        // hides dotfiles when the panel is asked to.
        self.filter.apply(&mut listing.entries);
        self.filter_listing(&mut listing);
        sort_entries(&mut listing.entries, self.sort_mode, self.sort_reversed);
        let mut entries = Vec::with_capacity(listing.entries.len() + 1);
        entries.extend(listing.parent);
        entries.append(&mut listing.entries);
        self.entries = entries;
        self.metadata = listing.metadata;
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
        self.pending_visit = Some(path.clone());
        if self.navigation_history.last() != Some(&self.path) {
            self.navigation_history.push(self.path.clone());
        }
        if self.navigation_history.len() > 100 {
            self.navigation_history.remove(0);
        }
        self.path = path;
        // Marks are names in the folder being left. A namesake in the new one
        // is a different file, so neither set may follow the panel there.
        self.marked.clear();
        self.previous_marks.clear();
        self.apply_listing(listing);
        self.ensure_selection();
    }

    /// The hidden attribute captured when this directory was loaded.
    pub(crate) fn is_hidden(&self, name: &str) -> bool {
        self.metadata.hidden_names.contains(name)
    }

    /// Writable space captured with this listing, or unknown.
    pub(crate) const fn available_space(&self) -> Option<FileSize> {
        self.metadata.available_space
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

    /// A panel over an already-known listing, touching no filesystem.
    ///
    /// [`Panel::new`] reads a real directory; this is for callers holding the
    /// entries already. It exists because `state` is private now, so a struct
    /// literal is no longer a way to build one — and the invariant is exactly
    /// what a literal used to be free to break.
    #[must_use]
    pub fn from_entries(path: PathBuf, entries: Vec<DirEntry>) -> Self {
        let panel_path = path.clone();
        let mut panel = Self {
            path,
            entries,
            state: ListState::default(),
            marked: BTreeSet::new(),
            previous_marks: BTreeSet::new(),
            sort_mode: SortMode::default(),
            sort_reversed: false,
            view: PanelView::default(),
            kind: PanelKind::default(),
            filter: PanelFilter::default(),
            description_names: crate::description::default_names(),
            // The starting directory counts as visited, as it does in Far.
            pending_visit: Some(panel_path),
            details: PanelDetails::default(),
            options: PanelOptions::default(),
            navigation_history: Vec::new(),
            metadata: PanelMetadata::default(),
        };
        panel.select(panel.entries.first().map(|_| 0));
        panel
    }

    #[allow(dead_code)]
    pub const fn cycle_sort_mode(&mut self) {
        self.sort_mode = self.sort_mode.next();
    }
}

/// Where a panel's cursor was before its listing was reloaded.
///
/// Both halves are needed, and they answer different questions. The name is
/// where the cursor belongs if the entry is still there — it may have moved
/// rows because something above it was created or removed. The index is where
/// it belongs if the entry is *not* still there, which is what happens after a
/// delete: Far leaves the cursor in place, on whatever now occupies that row.
pub struct Cursor {
    name: Option<String>,
    index: usize,
}

#[cfg(test)]
mod tests;
