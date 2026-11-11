//! Purpose: One entry whose destination is already taken, and the size and
//! time of both sides, for the "File already exists" question.

use std::path::{Path, PathBuf};
use std::time::SystemTime;

/// What one side of a clash is like: enough to tell which one to keep.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EntryFacts {
    /// Bytes, for a file; `None` for a folder or what could not be read.
    pub size: Option<u64>,
    pub modified: Option<SystemTime>,
    pub is_folder: bool,
}

impl EntryFacts {
    /// What `path` is like, following a link as the copy itself would. A
    /// link that points nowhere still holds the name, so its own facts stand
    /// in for the target's.
    #[must_use]
    pub fn of(path: &Path) -> Self {
        let Ok(meta) = std::fs::metadata(path).or_else(|_| std::fs::symlink_metadata(path)) else {
            return Self {
                size: None,
                modified: None,
                is_folder: false,
            };
        };
        Self {
            size: meta.is_file().then_some(meta.len()),
            modified: meta.modified().ok(),
            is_folder: meta.is_dir(),
        }
    }
}

/// One entry whose destination is already taken.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Clash {
    pub source: PathBuf,
    pub dest: PathBuf,
    /// The entry being sent.
    pub incoming: EntryFacts,
    /// What is already at `dest`.
    pub existing: EntryFacts,
}

impl Clash {
    #[must_use]
    pub fn between(source: &Path, dest: &Path) -> Self {
        Self {
            source: source.to_path_buf(),
            dest: dest.to_path_buf(),
            incoming: EntryFacts::of(source),
            existing: EntryFacts::of(dest),
        }
    }
}
