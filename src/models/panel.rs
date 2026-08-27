//! File panel state: directory listing, selection, marks, quick search.

use ratatui::widgets::ListState;
use std::{
    cmp::Ordering,
    collections::BTreeSet,
    collections::HashSet,
    fs, io,
    path::{Path, PathBuf},
};

use super::{DirEntry, EntryKind, FileName, FileSize, SortMode};

/// One panel row per directory entry.
///
/// `fs::DirEntry::metadata` is an `lstat`: it reports a symlink's own size and
/// says "not a directory" for a link pointing at one. Every consumer disagreed —
/// `Panel::enter` refused to descend while F5/F6/F8 resolved the link — so a
/// symlinked directory rendered as a tiny file that could not be opened but
/// whose whole target tree got copied. `is_dir` therefore describes the
/// *target*, and `is_symlink` preserves the fact that a link is involved.
/// A broken link keeps `is_dir == false`: there is nothing to enter.
fn read_dir_entries(path: &Path) -> io::Result<Vec<DirEntry>> {
    Ok(fs::read_dir(path)?
        .filter_map(|entry| {
            let entry = entry.ok()?;
            let metadata = entry.metadata().ok()?;
            // Reads exactly like the rule it used to need a paragraph for:
            // a link is classified by what is at the other end of it.
            let kind = if metadata.is_symlink() {
                match fs::metadata(entry.path()) {
                    Ok(target) if target.is_dir() => EntryKind::SymlinkToDirectory,
                    Ok(_) => EntryKind::SymlinkToFile,
                    Err(_) => EntryKind::BrokenSymlink,
                }
            } else if metadata.is_dir() {
                EntryKind::Directory
            } else {
                EntryKind::File
            };
            Some(DirEntry {
                name: entry.file_name().to_string_lossy().into_owned(),
                kind,
                size: FileSize::from_bytes(metadata.len()),
            })
        })
        .collect())
}

/// Directories first, then the mode-specific ordering.
fn sort_entries(entries: &mut [DirEntry], sort_mode: SortMode) {
    match sort_mode {
        SortMode::Name => {
            entries.sort_by_cached_key(|e| (!e.kind.is_dir(), e.name.to_lowercase()));
        }
        SortMode::Extension => {
            entries.sort_by_cached_key(|e| {
                let ext = Path::new(&e.name)
                    .extension()
                    .and_then(|x| x.to_str())
                    .unwrap_or("")
                    .to_lowercase();
                (!e.kind.is_dir(), ext, e.name.to_lowercase())
            });
        }
        SortMode::Size => {
            entries.sort_unstable_by(|a, b| match (a.kind.is_dir(), b.kind.is_dir()) {
                (true, false) => Ordering::Less,
                (false, true) => Ordering::Greater,
                _ => b
                    .size
                    .cmp(&a.size)
                    .then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase())),
            });
        }
    }
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
    pub sort_mode: SortMode,
}

impl Panel {
    pub fn new(path: PathBuf) -> io::Result<Self> {
        let mut panel = Self {
            path,
            entries: Vec::new(),
            state: ListState::default(),
            marked: BTreeSet::new(),
            sort_mode: SortMode::default(),
        };
        panel.load_entries()?;
        panel.select(panel.entries.first().map(|_| 0));
        Ok(panel)
    }

    /// Re-read the directory into `self.entries`.
    ///
    /// The whole listing is built before anything is published: clearing
    /// `self.entries` up front meant a failing `read_dir` blanked the panel down
    /// to a lone "..", throwing away the last good listing it could still have
    /// shown. On error the panel is left exactly as it was.
    pub fn load_entries(&mut self) -> io::Result<()> {
        let selected_name = self.get_selected_entry().map(|entry| entry.name.clone());

        let mut entries = read_dir_entries(&self.path)?;
        sort_entries(&mut entries, self.sort_mode);

        let mut listing = Vec::with_capacity(entries.len() + 1);
        if self.path.parent().is_some() {
            listing.push(DirEntry {
                name: "..".to_owned(),
                kind: EntryKind::Directory,
                ..DirEntry::default()
            });
        }
        listing.append(&mut entries);
        self.entries = listing;

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

        self.select_after_reload(selected_name);

        Ok(())
    }

    pub(crate) fn select_after_reload(&mut self, selected_name: Option<String>) {
        if let Some(selected_name) = selected_name {
            if let Some(index) = self
                .entries
                .iter()
                .position(|entry| entry.name == selected_name)
            {
                self.select(Some(index));
            } else if self.entries.is_empty() {
                self.select(None);
            } else {
                self.select(Some(0));
            }
        } else if self.entries.is_empty() {
            self.select(None);
        }
    }

    pub fn next(&mut self) {
        if self.entries.is_empty() {
            return;
        }
        let i = match self.state.selected() {
            Some(i) => {
                if i >= self.entries.len() - 1 {
                    0
                } else {
                    i + 1
                }
            }
            None => 0,
        };
        self.select(Some(i));
    }

    pub fn previous(&mut self) {
        if self.entries.is_empty() {
            return;
        }
        let i = match self.state.selected() {
            Some(i) => {
                if i == 0 {
                    self.entries.len() - 1
                } else {
                    i - 1
                }
            }
            None => 0,
        };
        self.select(Some(i));
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

    /// Move the panel to `path`, dropping the marks that belonged to the old
    /// listing.
    ///
    /// `marked` holds bare file *names*, so a mark only means anything relative
    /// to one directory: moving the path without clearing them silently re-binds
    /// every surviving name to a different file (mark `tmp` under `/home/me`,
    /// jump to `/`, and the next F6/F8 targets `/tmp`). Tying the two mutations
    /// together is what keeps that impossible — always navigate through here.
    ///
    /// A failed load rolls everything back: a panel must never show a path it
    /// has no listing for.
    ///
    /// "Always navigate through here" is not a suggestion: `:cd`, `:bookmark
    /// go` and both drive-menu paths each hand-rolled `clear_marks();
    /// panel.path = …; load_entries()` and each stranded the panel on an
    /// unreadable directory. `tests_panel_navigation_invariant` fails the build
    /// if a fifth site appears.
    pub fn navigate_to(&mut self, path: PathBuf) -> io::Result<()> {
        let previous_path = std::mem::replace(&mut self.path, path);
        let previous_selection = self.state.selected();
        let previous_marks = std::mem::take(&mut self.marked);

        let Err(error) = self.load_entries() else {
            self.ensure_selection();
            return Ok(());
        };

        self.path = previous_path;
        self.marked = previous_marks;
        let _ = self.load_entries(); // best effort: bring the old listing back
        self.select(previous_selection);
        self.ensure_selection();
        Err(error)
    }

    pub fn enter(&mut self) -> io::Result<()> {
        let Some(i) = self.state.selected() else {
            return Ok(());
        };
        let Some(entry) = self.entries.get(i) else {
            return Ok(());
        };
        if !entry.kind.is_dir() {
            return Ok(());
        }

        if entry.name == ".." {
            // Remember the directory we're leaving so we can select it after going up
            let prev_dir_name = self
                .path
                .file_name()
                .map(|n| n.to_string_lossy().to_string());
            let mut parent = self.path.clone();
            parent.pop();
            self.navigate_to(parent)?;
            // Select the directory we just came from
            if let Some(name) = prev_dir_name {
                if let Some(pos) = self.entries.iter().position(|e| e.name == name) {
                    self.select(Some(pos));
                    return Ok(());
                }
            }
            self.select(self.entries.first().map(|_| 0));
        } else {
            // Build the target first: the listing (and the entry we selected in
            // it) must stay valid until `navigate_to` has proven it readable.
            let target = self.path.join(&entry.name);
            self.navigate_to(target)?;
            self.select(self.entries.first().map(|_| 0));
        }
        Ok(())
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
        let mut panel = Self {
            path,
            entries,
            state: ListState::default(),
            marked: BTreeSet::new(),
            sort_mode: SortMode::default(),
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
        self.marked.clear();
    }

    /// Find and select the next entry matching the given prefix (case-insensitive).
    /// If already on a matching entry, moves to the next matching one.
    /// Returns true if a match was found.
    pub fn find_next_matching(&mut self, prefix: &str) -> bool {
        if self.entries.is_empty() || prefix.is_empty() {
            return false;
        }

        let prefix_lower = prefix.to_lowercase();
        let current = self.state.selected().unwrap_or(0);
        let len = self.entries.len();

        // Search from current+1 to end, then from 0 to current (inclusive)
        for offset in 1..=len {
            let idx = (current + offset) % len;
            let entry = &self.entries[idx];
            if entry.name != ".." && entry.name.to_lowercase().starts_with(&prefix_lower) {
                self.select(Some(idx));
                return true;
            }
        }

        false
    }

    /// Find and select the first entry matching the given prefix (case-insensitive).
    /// Returns true if a match was found.
    pub fn find_first_matching(&mut self, prefix: &str) -> bool {
        if self.entries.is_empty() || prefix.is_empty() {
            return false;
        }

        let prefix_lower = prefix.to_lowercase();

        for (idx, entry) in self.entries.iter().enumerate() {
            if entry.name != ".." && entry.name.to_lowercase().starts_with(&prefix_lower) {
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
