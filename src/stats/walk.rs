//! Purpose: The bounded walk that measures a tree, and the running tally it
//! folds each subtree into.

use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

use super::{
    ChildSize, ContentKind, Coverage, DirectoryStats, KindTotal, LargeFile, CHILDREN_SHOWN,
    LARGEST_SHOWN,
};
use crate::models::FileSize;

/// One subtree's contribution, before it is folded into the parent's totals.
#[derive(Default)]
pub(super) struct Tally {
    size: u64,
    files: usize,
    directories: usize,
    /// Folders the walk could not list.
    pub(super) unread: usize,
    by_kind: BTreeMap<ContentKind, (u64, usize)>,
    largest: Vec<LargeFile>,
}

impl Tally {
    pub(super) fn absorb(&mut self, other: Self) {
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

    const fn child(&self, name: String) -> ChildSize {
        ChildSize {
            name,
            size: FileSize::from_bytes(self.size),
            files: self.files,
        }
    }
}

/// Trim a running list of large files back to what will be shown.
///
/// Done as the walk goes rather than at the end: a tree with a million files
/// would otherwise be sorted whole to report eight of them.
fn keep_largest(files: &mut Vec<LargeFile>) {
    files.sort_by_key(|file| std::cmp::Reverse(file.size));
    files.truncate(LARGEST_SHOWN);
}

/// The walk itself, with the budget passed in.
///
/// Split out so the exhausted case can be reached without building two hundred
/// thousand files: "the numbers are a lower bound" is a distinct answer the
/// panel prints, and one nothing could otherwise check.
pub(super) fn analyse_within(directory: &Path, mut budget: usize) -> DirectoryStats {
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
        // `metadata` on a `DirEntry` does not follow symlinks, which is what
        // keeps `a -> ..` from walking forever.
        let Ok(meta) = entry.metadata() else { continue };
        let name = entry.file_name();
        if meta.is_dir() {
            let name = name.to_string_lossy().into_owned();
            let mut sub = Tally::default();
            walk(&entry.path(), &name, &mut sub, &mut budget);
            children.push(sub.child(name));
            total.directories += 1;
            total.absorb(sub);
        } else if meta.is_file() {
            record_file(&mut total, &name.to_string_lossy(), None, meta.len());
        } else if is_special(&meta) {
            total.files += 1;
        }
    }
    summarize(total, children, budget)
}

pub(super) fn summarize(
    mut total: Tally,
    mut children: Vec<ChildSize>,
    budget: usize,
) -> DirectoryStats {
    children.sort_by(|a, b| b.size.cmp(&a.size).then_with(|| a.name.cmp(&b.name)));
    children.truncate(CHILDREN_SHOWN);

    let by_kind = kind_totals(&total);
    keep_largest(&mut total.largest);

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
pub(super) const fn coverage(budget: usize, unread: usize) -> Coverage {
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

/// Add one file to a tally, building its relative path only when it qualifies.
fn record_file(tally: &mut Tally, name: &str, prefix: Option<&str>, size: u64) {
    tally.size += size;
    tally.files += 1;
    let entry = tally.by_kind.entry(ContentKind::of(name)).or_default();
    entry.0 += size;
    entry.1 += 1;
    let size = FileSize::from_bytes(size);
    if tally.largest.len() < LARGEST_SHOWN || tally.largest.last().is_none_or(|f| size > f.size) {
        let path = prefix.map_or_else(|| name.to_owned(), |dir| format!("{dir}/{name}"));
        tally.largest.push(LargeFile { path, size });
        keep_largest(&mut tally.largest);
    }
}

/// Walk one subtree, spending from the shared budget.
pub(super) fn walk(directory: &Path, prefix: &str, tally: &mut Tally, budget: &mut usize) {
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
        let raw_name = entry.file_name();
        let name = raw_name.to_string_lossy();

        if meta.is_dir() {
            tally.directories += 1;
            walk(&entry.path(), &format!("{prefix}/{name}"), tally, budget);
        } else if meta.is_file() {
            record_file(tally, &name, Some(prefix), meta.len());
        } else if is_special(&meta) {
            tally.files += 1;
        }
    }
}

/// A socket, a device node or a FIFO: counted as a file, as the panel counts
/// it, but with no data of its own for a size, a category or the largest
/// files. A link is not one: its target is counted where it lives.
fn is_special(meta: &fs::Metadata) -> bool {
    !meta.is_dir() && !meta.is_file() && !meta.is_symlink()
}
