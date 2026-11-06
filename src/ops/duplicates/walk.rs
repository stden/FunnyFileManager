//! Purpose: the folder walk behind Find duplicate blocks: which text files are
//! read, and the budget that stops the walk before it takes too long or too much.

use std::{
    fs,
    path::{Path, PathBuf},
    time::{Duration, Instant},
};

use crate::{
    fileload::{read_for_text_ui, Loaded},
    mask::Mask,
    progress,
};

/// Most files one search reads.
pub(super) const FILE_LIMIT: usize = 5_000;

/// Most bytes one search reads, in all.
pub(super) const BYTE_LIMIT: u64 = 64 * 1024 * 1024;

/// Longest a search may run, from the moment it starts.
pub(super) const TIME_LIMIT: Duration = Duration::from_secs(10);

/// Largest file read; a bigger one is generated output or a log.
pub(super) const MAX_FILE_BYTES: u64 = 1024 * 1024;

/// Build output and dependencies: a copy in them is nobody's source.
const SKIPPED_FOLDERS: [&str; 4] = ["target", "node_modules", "__pycache__", "venv"];

/// Which budget ended a walk before it had looked everywhere.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Limit {
    Files,
    Time,
}

/// What one walk may read before it must stop.
#[derive(Debug, Clone, Copy)]
pub(super) struct Budget {
    pub files: usize,
    pub bytes: u64,
    pub deadline: Instant,
}

impl Budget {
    /// The budget a search from the dialog gets.
    pub(super) fn standard() -> Self {
        Self {
            files: FILE_LIMIT,
            bytes: BYTE_LIMIT,
            deadline: Instant::now() + TIME_LIMIT,
        }
    }
}

/// The text files a walk read, in path order, and the limit that stopped it.
pub(super) struct Walk {
    pub files: Vec<(PathBuf, String)>,
    pub stopped: Option<Limit>,
}

pub(super) fn walk(root: &Path, mask: &Mask, budget: Budget) -> Walk {
    let mut walker = Walker {
        mask,
        budget,
        files: Vec::new(),
        bytes: 0,
        stopped: None,
    };
    walker.visit(root);
    walker.files.sort_by(|left, right| left.0.cmp(&right.0));
    Walk {
        files: walker.files,
        stopped: walker.stopped,
    }
}

/// One walk in progress: what it has read so far, and whether a limit stopped it.
struct Walker<'a> {
    mask: &'a Mask,
    budget: Budget,
    files: Vec<(PathBuf, String)>,
    bytes: u64,
    stopped: Option<Limit>,
}

impl Walker<'_> {
    /// The folder's entries in name order. Symbolic links are never followed.
    fn visit(&mut self, folder: &Path) {
        progress::entering(folder);
        let Ok(listing) = fs::read_dir(folder) else {
            return;
        };
        let mut entries: Vec<fs::DirEntry> = listing.filter_map(Result::ok).collect();
        entries.sort_by_key(fs::DirEntry::file_name);
        for entry in &entries {
            self.entry(entry);
        }
    }

    fn entry(&mut self, entry: &fs::DirEntry) {
        if self.stopped.is_some() {
            return;
        }
        if Instant::now() >= self.budget.deadline {
            self.stopped = Some(Limit::Time);
            return;
        }
        progress::examined();
        let name = entry.file_name().to_string_lossy().into_owned();
        if name.starts_with('.') {
            return;
        }
        self.by_kind(&entry.path(), &name, entry.file_type().ok());
    }

    /// Read a file or enter a folder. A link is never followed, and an entry
    /// whose kind could not be read is left alone.
    fn by_kind(&mut self, path: &Path, name: &str, kind: Option<fs::FileType>) {
        let Some(kind) = kind else {
            return;
        };
        if kind.is_symlink() {
            return;
        }
        if !kind.is_dir() {
            self.file(path, name);
        } else if !SKIPPED_FOLDERS.contains(&name) {
            self.visit(path);
        }
    }

    fn file(&mut self, path: &Path, name: &str) {
        if !self.mask.matches(name) {
            return;
        }
        if self.files.len() >= self.budget.files || self.bytes >= self.budget.bytes {
            self.stopped = Some(Limit::Files);
            return;
        }
        let Some(text) = read_text(path) else {
            return;
        };
        self.bytes = self
            .bytes
            .saturating_add(u64::try_from(text.len()).unwrap_or(u64::MAX));
        progress::matched();
        self.files.push((path.to_path_buf(), text));
    }
}

/// The file's text, when it is a regular file of at most [`MAX_FILE_BYTES`]
/// whose bytes are UTF-8 with no NUL: anything else is not source text.
fn read_text(path: &Path) -> Option<String> {
    let size = fs::metadata(path).ok()?.len();
    if size > MAX_FILE_BYTES {
        return None;
    }
    let Loaded::Bytes(bytes) = read_for_text_ui(path).ok()? else {
        return None;
    };
    let text = String::from_utf8(bytes).ok()?;
    (!text.contains('\0')).then_some(text)
}

#[cfg(test)]
mod tests;
