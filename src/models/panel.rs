//! File panel state: directory snapshots, selection, marks and quick search.
//! Filesystem loading and navigation effects live in `ops::panel`.

use ratatui::widgets::ListState;
use std::{
    cell::RefCell,
    collections::{BTreeSet, VecDeque},
    path::PathBuf,
};

use super::{
    DirEntry, FileName, FileSize, PanelFilter, PanelKind, ShortNames, SortMode, SortOrder,
};

mod collection;
mod cursor;
mod dirty;
mod listing;
mod marks;
mod search;
mod search_index;
mod sort;
mod totals;
pub use collection::PanelCollection;
pub mod view;
pub use dirty::DirtyProbe;
pub use view::{PanelDetails, PanelOptions, PanelView};

/// Filesystem facts captured with a directory listing.
///
/// Keeping these on the panel preserves existing public `DirEntry` literals.
/// Bare-name keys remain stable when the entries are sorted.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PanelMetadata {
    pub(crate) hidden_names: BTreeSet<FileName>,
    pub(crate) dirty_repos: BTreeSet<FileName>,
    pub(crate) dirty_probe: DirtyProbe,
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
    pub entries: PanelCollection<Vec<DirEntry>>,
    /// Private: the highlighted row and the entry list share an invariant —
    /// the index must name a row that exists. It was enforced by hand at every
    /// call site, including a written-out `filter(|i| *i < entries.len())`.
    /// [`Panel::select`] does it once instead.
    state: ListState,
    pub marked: PanelCollection<BTreeSet<FileName>>,
    /// What was marked before the last change to `marked`, so Ctrl+M can put
    /// it back. Far calls it "restore selection", and it is what saves a user
    /// who has just typed the wrong mask over a careful hand-made selection.
    /// Like `marked`, it only ever names entries of the current listing.
    previous_marks: PanelCollection<BTreeSet<FileName>>,
    pub sort_mode: SortMode,
    /// `-` on the sort menu: whether the chosen order runs direct or backwards.
    pub sort_order: SortOrder,
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
    pub navigation_history: VecDeque<PathBuf>,
    /// Facts captured with the listing, never queried while rendering.
    metadata: PanelMetadata,
    /// Cached 8.3 short-name table for the current listing order.
    short_names: RefCell<Option<((usize, usize), ShortNames)>>,
    /// Folded-name index for quick search. Rebuilt when the listing moves.
    name_index: RefCell<Option<search_index::NameIndex>>,
    totals: totals::PanelTotals,
}

impl Panel {
    /// The hidden attribute captured when this directory was loaded.
    pub(crate) fn is_hidden(&self, name: &str) -> bool {
        self.metadata.hidden_names.contains(name)
    }

    /// Whether a directory entry is a Git repository with uncommitted changes.
    pub(crate) fn is_dirty_repo(&self, name: &str) -> bool {
        self.metadata.dirty_repos.contains(name)
    }

    /// Writable space captured with this listing, or unknown.
    pub(crate) const fn available_space(&self) -> Option<FileSize> {
        self.metadata.available_space
    }

    /// Cached 8.3 forms for the current listing, built on first draw and
    /// reused across frames until the listing or its sort order changes.
    #[must_use]
    pub(crate) fn short_names(&self) -> ShortNames {
        let key = (self.entries.as_ptr().addr(), self.entries.len());
        if let Some((_, table)) = self
            .short_names
            .borrow()
            .as_ref()
            .filter(|(cached_key, _)| *cached_key == key)
        {
            return table.clone();
        }
        let table = ShortNames::for_entries(&self.entries);
        *self.short_names.borrow_mut() = Some((key, table.clone()));
        table
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
            entries: entries.into(),
            state: ListState::default(),
            marked: PanelCollection::default(),
            previous_marks: PanelCollection::default(),
            sort_mode: SortMode::default(),
            sort_order: SortOrder::Direct,
            view: PanelView::default(),
            kind: PanelKind::default(),
            filter: PanelFilter::default(),
            description_names: crate::description::default_names(),
            // The starting directory counts as visited, as it does in Far.
            pending_visit: Some(panel_path),
            details: PanelDetails::default(),
            options: PanelOptions::default(),
            navigation_history: VecDeque::new(),
            metadata: PanelMetadata::default(),
            short_names: RefCell::new(None),
            name_index: RefCell::new(None),
            totals: totals::PanelTotals::default(),
        };
        panel.select(panel.entries.first().map(|_| 0));
        panel
    }

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
#[cfg(test)]
mod tests_cursor_motion;
#[cfg(test)]
mod tests_dirty;
#[cfg(test)]
mod tests_search_order;
#[cfg(test)]
mod tests_sort_order;
