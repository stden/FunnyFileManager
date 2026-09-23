//! Purpose: Far's view/edit and folder histories — the files F3 and F4 opened
//! and the directories the panels have been in, remembered across runs.
//!
//! Both are the same shape: a bounded, ordered list of paths where touching a
//! path moves it to the end. They differ only in what they are for and which
//! file holds them, so one generic body carries a marker for each — two
//! distinct types, because a viewer list handed to the folder dialog is a bug
//! the compiler can catch.

use std::{
    collections::VecDeque,
    fs, io,
    marker::PhantomData,
    path::{Path, PathBuf},
};

use crate::history::Persistence;

/// Most paths a history keeps; the oldest fall off first. Far's default is
/// 512, more than a menu can show.
pub const PATH_HISTORY_CAP: usize = 200;

/// Where one history persists; each marker names its own file.
pub trait HistoryFile {
    /// The file under the data directory, beside the command history.
    const FILE: &'static str;
}

/// Files opened by F3, F4 and Alt+F4 — Alt+F11.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Viewed;

impl HistoryFile for Viewed {
    const FILE: &'static str = "view_history";
}

/// Directories a panel has shown — Alt+F12.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Visited;

impl HistoryFile for Visited {
    const FILE: &'static str = "folder_history";
}

pub type ViewHistory = PathHistory<Viewed>;
pub type FolderHistory = PathHistory<Visited>;

/// A bounded list of paths, most recent last, that may persist to one file.
///
/// `persistence` follows the command history's rule: `default()` is
/// session-only, so constructing an `App` never touches the user's real
/// files; only [`PathHistory::load`] at the production entry point names a
/// file, and only [`PathHistory::save`] at exit writes it.
#[derive(Debug, Clone)]
pub struct PathHistory<K: HistoryFile> {
    entries: VecDeque<PathBuf>,
    persistence: Persistence,
    kind: PhantomData<K>,
}

impl<K: HistoryFile> Default for PathHistory<K> {
    fn default() -> Self {
        Self {
            entries: VecDeque::new(),
            persistence: Persistence::SessionOnly,
            kind: PhantomData,
        }
    }
}

impl<K: HistoryFile> PathHistory<K> {
    /// Read the persisted list and remember where it came from. A missing or
    /// unreadable file is an empty history; a history must never break the
    /// file manager.
    #[must_use]
    pub fn load() -> Self {
        let persistence =
            crate::history::data_file(K::FILE).map_or(Persistence::SessionOnly, Persistence::File);
        let entries = persistence
            .file()
            .and_then(|path| fs::read_to_string(path).ok())
            .map(|text| {
                text.lines()
                    .filter(|line| !line.is_empty())
                    .map(PathBuf::from)
                    .collect::<VecDeque<_>>()
            })
            .unwrap_or_default();
        let mut history = Self {
            entries,
            persistence,
            kind: PhantomData,
        };
        history.trim();
        history
    }

    /// Remember `path` as the most recent, moving it from wherever it was.
    /// A path with a line break in it cannot be one line of the file, so it
    /// is not remembered at all rather than written as two.
    pub fn touch(&mut self, path: &Path) {
        if path.to_string_lossy().contains(['\n', '\r']) {
            return;
        }
        self.entries.retain(|entry| entry != path);
        self.entries.push_back(path.to_owned());
        self.trim();
    }

    fn trim(&mut self) {
        while self.entries.len() > PATH_HISTORY_CAP {
            self.entries.pop_front();
        }
    }

    /// Oldest first, most recent last — file order.
    pub fn entries(&self) -> impl Iterator<Item = &Path> {
        self.entries.iter().map(PathBuf::as_path)
    }

    /// Most recent first — the order the history dialogs list.
    pub fn newest_first(&self) -> impl Iterator<Item = &Path> {
        self.entries.iter().rev().map(PathBuf::as_path)
    }

    #[must_use]
    pub fn last(&self) -> Option<&Path> {
        self.entries.back().map(PathBuf::as_path)
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Write the list, one path per line, where [`Self::load`] read it. A
    /// no-op for a session-only history.
    pub fn save(&self) -> io::Result<()> {
        let Some(path) = self.persistence.file() else {
            return Ok(());
        };
        if let Some(dir) = path.parent() {
            fs::create_dir_all(dir)?;
        }
        let mut text = String::new();
        for entry in &self.entries {
            text.push_str(&entry.to_string_lossy());
            text.push('\n');
        }
        crate::history::write_owner_only(path, &text)
    }
}

#[cfg(test)]
mod tests;
