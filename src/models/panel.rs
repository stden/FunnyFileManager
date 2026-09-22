//! File panel state: directory snapshots, selection, marks and quick search.
//! Filesystem loading and navigation effects live in `ops::panel`.

use ratatui::widgets::ListState;
use std::{
    cmp::Reverse,
    collections::{BTreeSet, HashSet},
    path::{Path, PathBuf},
};

use super::{DirEntry, FileName, FileSize, PanelFilter, PanelKind, SortMode};
use crate::casefold::starts_with_ignore_case;

pub mod view;
pub use view::{PanelDetails, PanelOptions, PanelView};

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

/// Directories first, then the mode's own order within each group; `reversed`
/// turns each group around but keeps the directories on top, as Far does.
fn sort_entries(entries: &mut [DirEntry], sort_mode: SortMode, reversed: bool) {
    match sort_mode {
        // Stable, so the listing's own order survives within each group.
        SortMode::Unsorted => entries.sort_by_key(|e| !e.kind.is_dir()),
        SortMode::Name => sort_by(entries, |_| ()),
        SortMode::Extension => sort_by(entries, |e| extension(&e.name)),
        SortMode::Size => sort_by(entries, |e| Reverse(e.size)),
        SortMode::ModificationTime => sort_by(entries, |e| Reverse(e.modified)),
        SortMode::CreationTime => sort_by(entries, |e| Reverse(e.created)),
        SortMode::AccessTime => sort_by(entries, |e| Reverse(e.accessed)),
        SortMode::Descriptions => sort_by(entries, |e| lowercase(e.description.as_deref())),
        SortMode::Owner => sort_by(entries, |e| lowercase(e.owner.as_deref())),
    }
    if reversed {
        let directories = entries.iter().take_while(|e| e.kind.is_dir()).count();
        let (top, rest) = entries.split_at_mut(directories);
        top.reverse();
        rest.reverse();
    }
}

/// Directories first, then `key`, then the lowercased name as the tie-break.
///
/// `sort_by_cached_key` rather than a comparator, and for one reason: the
/// tie-break lowercases the name, and doing that inside a comparator
/// allocates two `String`s per comparison — about a quarter of a million of
/// them for a directory of ten thousand files. Cached, each name is
/// lowercased once.
fn sort_by<K: Ord>(entries: &mut [DirEntry], key: impl Fn(&DirEntry) -> K) {
    entries.sort_by_cached_key(|e| (!e.kind.is_dir(), key(e), e.name.to_lowercase()));
}

fn extension(name: &str) -> String {
    Path::new(name)
        .extension()
        .and_then(|x| x.to_str())
        .unwrap_or("")
        .to_lowercase()
}

/// Rows without the fact sort together, before those that have it.
fn lowercase(text: Option<&str>) -> String {
    text.unwrap_or_default().to_lowercase()
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

        if !self.marked.is_empty() {
            let valid_names: HashSet<&str> = self
                .entries
                .iter()
                .filter(|entry| entry.name != "..")
                .map(|entry| entry.name.as_str())
                .collect();
            self.marked
                .retain(|name| valid_names.contains(name.as_str()));
        }
        self.select_after_reload(&previous);
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
        self.marked.clear();
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

    /// The highlighted row, if any.
    #[must_use]
    pub fn selected(&self) -> Option<usize> {
        self.state.selected()
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
                Some(self.path.join(&entry.name))
            }
        })
    }

    pub fn toggle_mark_selected(&mut self) -> bool {
        let Some(selected_name) = self.get_selected_entry().map(|entry| entry.name.clone()) else {
            return false;
        };

        if selected_name == ".." {
            return false;
        }

        if self.marked.contains(selected_name.as_str()) {
            self.marked.remove(selected_name.as_str());
        } else {
            self.marked.insert(FileName::new(selected_name));
        }

        true
    }

    pub fn clear_marks(&mut self) {
        self.remember_marks();
        self.marked.clear();
    }

    /// Keep the current selection for Ctrl+M, before replacing it.
    pub(crate) fn remember_marks(&mut self) {
        self.previous_marks.clone_from(&self.marked);
    }

    /// Every entry this panel can act on — the listing without its `..` row,
    /// which is not a file and must never end up marked.
    fn markable(&self) -> impl Iterator<Item = &str> {
        self.entries
            .iter()
            .map(|entry| entry.name.as_str())
            .filter(|name| *name != "..")
    }

    /// Gray + / Gray −: add every entry matching `mask` to the selection, or
    /// take every one of them out of it. Returns how many rows changed, which
    /// is what the status line reports.
    pub fn mark_matching(&mut self, mask: &crate::mask::Mask, select: bool) -> usize {
        self.remember_marks();
        // Collected before anything is marked: the scan borrows the listing,
        // and the marking mutates the set sitting beside it.
        let hits: Vec<FileName> = self
            .markable()
            .filter(|name| mask.matches(name))
            .map(|name| FileName::new(name.to_owned()))
            .collect();
        let mut changed = 0;
        for name in hits {
            let moved = if select {
                self.marked.insert(name)
            } else {
                self.marked.remove(name.as_str())
            };
            changed += usize::from(moved);
        }
        changed
    }

    /// Gray ∗: everything selected becomes unselected and the other way round.
    pub fn invert_marks(&mut self) {
        self.remember_marks();
        let inverted: BTreeSet<FileName> = self
            .markable()
            .filter(|name| !self.marked.contains(*name))
            .map(|name| FileName::new(name.to_owned()))
            .collect();
        self.marked = inverted;
    }

    /// Ctrl+M: put back the selection as it was before the last change.
    ///
    /// A swap rather than an assignment, so pressing it twice returns to where
    /// it started — the undo of an undo, which is what a user who pressed it
    /// by mistake expects.
    pub fn restore_marks(&mut self) {
        std::mem::swap(&mut self.marked, &mut self.previous_marks);
    }

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
            let entry = &self.entries[idx];
            if entry.name != ".." && starts_with_ignore_case(&entry.name, prefix) {
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

    #[allow(dead_code)]
    pub fn action_entry_names(&self) -> Vec<String> {
        if !self.marked.is_empty() {
            return self
                .marked
                .iter()
                .map(FileName::as_str)
                .map(str::to_owned)
                .collect();
        }

        self.get_selected_entry()
            .and_then(|entry| {
                if entry.name == ".." {
                    None
                } else {
                    Some(entry.name.clone())
                }
            })
            .into_iter()
            .collect()
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
