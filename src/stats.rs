//! Purpose: What a directory actually contains — where the size is, and what kind.
//!
//! Two questions a file manager is asked constantly and usually cannot answer:
//! *which subdirectory is eating the space*, and *what is all this*. The panel
//! shows one directory level and a size per file, which answers neither.
//!
//! The walk is bounded and does not follow symlinks. A statistics view that can
//! hang the program on a deep tree, or loop forever on `a -> ..`, is worse than
//! no statistics view: the user pressed a key to look at something, not to lose
//! the session.

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant, SystemTime};

use crate::models::FileSize;

mod budget;
mod kind;
mod measure;
mod walk;

pub use budget::{Budget, Stop};
pub use kind::ContentKind;
pub use measure::{measure_entries, Measured, Shape};
use walk::analyse_within;

/// Entries the walk will visit before giving up and saying so.
///
/// Large enough for a source tree, small enough that the pause is a blink on a
/// spinning disk. [`DirectoryStats::complete`] reports whether it was hit, so a
/// partial answer is never presented as a total.
const WALK_BUDGET: usize = 50_000;

/// How long an unchanged directory's statistics are reused across repeated
/// `Ctrl+L`, `F3`, and quick-view inspections.
const STATS_CACHE_TTL: Duration = Duration::from_secs(30);

/// Immediate subdirectories reported, largest first.
const CHILDREN_SHOWN: usize = 12;

/// Individual files reported, largest first.
const LARGEST_SHOWN: usize = 8;

#[derive(Clone)]
struct CachedStats {
    modified: Option<SystemTime>,
    checked_at: Instant,
    stats: DirectoryStats,
}

fn stats_cache() -> &'static Mutex<HashMap<PathBuf, CachedStats>> {
    static CACHE: OnceLock<Mutex<HashMap<PathBuf, CachedStats>>> = OnceLock::new();
    CACHE.get_or_init(|| Mutex::new(HashMap::new()))
}

/// One immediate subdirectory, with everything beneath it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChildSize {
    pub name: String,
    pub size: FileSize,
    pub files: usize,
}

/// One content category's share of the tree.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct KindTotal {
    pub kind: ContentKind,
    pub size: FileSize,
    pub files: usize,
}

/// One large file, named so the user can go and look at it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LargeFile {
    /// Relative to the directory that was analysed.
    pub path: String,
    pub size: FileSize,
}

/// Whether the walk saw the whole tree.
///
/// Named answers rather than a `bool`, because the difference decides
/// whether the numbers may be called totals — and an unreadable folder's
/// zeros are not an empty folder's.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub enum Coverage {
    #[default]
    Complete,
    /// The budget ran out; every number is a lower bound.
    Partial,
    /// This many folders inside could not be read; every number is a lower
    /// bound.
    Unread(usize),
    /// The folder itself could not be read, and why: there are no numbers.
    Unreadable(String),
}

impl Coverage {
    #[must_use]
    pub const fn is_complete(&self) -> bool {
        matches!(self, Self::Complete)
    }
}

/// What a directory contains.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DirectoryStats {
    pub total: FileSize,
    pub files: usize,
    pub directories: usize,
    /// Immediate subdirectories, largest first.
    pub children: Vec<ChildSize>,
    /// Content categories present, largest first.
    pub by_kind: Vec<KindTotal>,
    /// The biggest individual files, largest first.
    pub largest: Vec<LargeFile>,
    pub coverage: Coverage,
}

impl DirectoryStats {
    /// The answer for a folder that could not be listed at all: no numbers,
    /// only why.
    fn unreadable(error: &std::io::Error) -> Self {
        Self {
            coverage: Coverage::Unreadable(error.to_string()),
            ..Self::default()
        }
    }
}

/// Measure everything under `directory`.
///
/// Never fails: an unreadable subdirectory contributes nothing and the rest of
/// the walk carries on. A statistics view that refuses to show anything because
/// one directory denied permission is answering a question nobody asked. The
/// coverage says what was not seen, though, so a denied folder is never shown
/// as an empty one.
#[must_use]
pub fn analyse(directory: &Path) -> DirectoryStats {
    let modified = fs::metadata(directory).and_then(|m| m.modified()).ok();
    let now = Instant::now();
    if let Some(cached) = stats_cache()
        .lock()
        .ok()
        .and_then(|guard| guard.get(directory).cloned())
        .filter(|entry| {
            entry.modified == modified
                && now.saturating_duration_since(entry.checked_at) < STATS_CACHE_TTL
        })
    {
        return cached.stats;
    }
    let stats = analyse_within(directory, Budget::new(WALK_BUDGET, None));
    if !matches!(stats.coverage, Coverage::Unreadable(_)) {
        if let Ok(mut guard) = stats_cache().lock() {
            guard.insert(
                directory.to_path_buf(),
                CachedStats {
                    modified,
                    checked_at: now,
                    stats: stats.clone(),
                },
            );
        }
    }
    stats
}

#[cfg(test)]
mod tests;
