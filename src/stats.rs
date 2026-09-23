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

use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

use crate::models::FileSize;

mod kind;

pub use kind::ContentKind;

/// Entries the walk will visit before giving up and saying so.
///
/// Large enough for a source tree, small enough that the pause is a blink on a
/// spinning disk. [`DirectoryStats::complete`] reports whether it was hit, so a
/// partial answer is never presented as a total.
const WALK_BUDGET: usize = 200_000;

/// Immediate subdirectories reported, largest first.
const CHILDREN_SHOWN: usize = 12;

/// Individual files reported, largest first.
const LARGEST_SHOWN: usize = 8;

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

/// One subtree's contribution, before it is folded into the parent's totals.
#[derive(Default)]
struct Tally {
    size: u64,
    files: usize,
    directories: usize,
    /// Folders the walk could not list.
    unread: usize,
    by_kind: BTreeMap<ContentKind, (u64, usize)>,
    largest: Vec<LargeFile>,
}

impl Tally {
    fn absorb(&mut self, other: Self) {
        self.size += other.size;
        self.files += other.files;
        self.directories += other.directories;
        self.unread += other.unread;
        for (kind, (size, files)) in other.by_kind {
            let entry = self.by_kind.entry(kind).or_default();
            entry.0 += size;
            entry.1 += files;
        }
        self.largest.extend(other.largest);
        keep_largest(&mut self.largest);
    }
}

/// Trim a running list of large files back to what will be shown.
///
/// Done as the walk goes rather than at the end: a tree with a million files
/// would otherwise be sorted whole to report eight of them.
fn keep_largest(files: &mut Vec<LargeFile>) {
    if files.len() <= LARGEST_SHOWN {
        return;
    }
    files.sort_by_key(|file| std::cmp::Reverse(file.size));
    files.truncate(LARGEST_SHOWN);
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
    analyse_within(directory, WALK_BUDGET)
}

/// The walk itself, with the budget passed in.
///
/// Split out so the exhausted case can be reached without building two hundred
/// thousand files: "the numbers are a lower bound" is a distinct answer the
/// panel prints, and one nothing could otherwise check.
fn analyse_within(directory: &Path, mut budget: usize) -> DirectoryStats {
    let mut children = Vec::new();
    let mut total = Tally::default();

    let entries = match fs::read_dir(directory) {
        Ok(entries) => entries,
        Err(error) => return DirectoryStats::unreadable(&error),
    };

    for entry in entries.flatten() {
        if budget == 0 {
            break;
        }
        budget -= 1;
        let name = entry.file_name().to_string_lossy().into_owned();
        // `metadata` on a `DirEntry` does not follow symlinks, which is what
        // keeps `a -> ..` from walking forever.
        let Ok(meta) = entry.metadata() else { continue };

        if meta.is_dir() {
            let mut subtree = Tally::default();
            walk(&entry.path(), &name, &mut subtree, &mut budget);
            children.push(ChildSize {
                name,
                size: FileSize::from_bytes(subtree.size),
                files: subtree.files,
            });
            total.directories += 1;
            total.absorb(subtree);
        } else if meta.is_file() {
            record_file(&mut total, &name, name.clone(), meta.len());
        }
    }

    summarize(total, children, budget)
}

fn summarize(mut total: Tally, mut children: Vec<ChildSize>, budget: usize) -> DirectoryStats {
    children.sort_by(|a, b| b.size.cmp(&a.size).then_with(|| a.name.cmp(&b.name)));
    children.truncate(CHILDREN_SHOWN);

    let by_kind = kind_totals(&total);

    keep_largest(&mut total.largest);
    total
        .largest
        .sort_by_key(|file| std::cmp::Reverse(file.size));

    DirectoryStats {
        total: FileSize::from_bytes(total.size),
        files: total.files,
        directories: total.directories,
        children,
        by_kind,
        largest: total.largest,
        coverage: coverage(budget, total.unread),
    }
}

/// What a walk that ended with `budget` left and `unread` folders unlisted saw.
const fn coverage(budget: usize, unread: usize) -> Coverage {
    if budget == 0 {
        Coverage::Partial
    } else if unread > 0 {
        Coverage::Unread(unread)
    } else {
        Coverage::Complete
    }
}

fn kind_totals(total: &Tally) -> Vec<KindTotal> {
    let mut by_kind: Vec<KindTotal> = total
        .by_kind
        .iter()
        .map(|(kind, (size, files))| KindTotal {
            kind: *kind,
            size: FileSize::from_bytes(*size),
            files: *files,
        })
        .collect();
    by_kind.sort_by(|a, b| b.size.cmp(&a.size).then_with(|| a.kind.cmp(&b.kind)));
    by_kind
}

/// Add one file to a tally.
fn record_file(tally: &mut Tally, name: &str, path: String, size: u64) {
    tally.size += size;
    tally.files += 1;
    let entry = tally.by_kind.entry(ContentKind::of(name)).or_default();
    entry.0 += size;
    entry.1 += 1;
    tally.largest.push(LargeFile {
        path,
        size: FileSize::from_bytes(size),
    });
    keep_largest(&mut tally.largest);
}

/// Walk one subtree, spending from the shared budget.
fn walk(directory: &Path, prefix: &str, tally: &mut Tally, budget: &mut usize) {
    let Ok(entries) = fs::read_dir(directory) else {
        tally.unread += 1;
        return;
    };
    for entry in entries.flatten() {
        if *budget == 0 {
            return;
        }
        *budget -= 1;
        let Ok(meta) = entry.metadata() else { continue };
        let name = entry.file_name().to_string_lossy().into_owned();
        let path = format!("{prefix}/{name}");

        if meta.is_dir() {
            tally.directories += 1;
            walk(&entry.path(), &path, tally, budget);
        } else if meta.is_file() {
            record_file(tally, &name, path, meta.len());
        }
    }
}

#[cfg(test)]
mod tests;
