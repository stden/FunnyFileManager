//! Purpose: The bounded walk that measures a tree, and the running tally it
//! folds each subtree into.

use std::fs;
use std::path::Path;

use super::measure::leaf;
use super::{
    Budget, ChildSize, ContentKind, Coverage, DirectoryStats, KindTotal, LargeFile, Shape, Stop,
    CHILDREN_SHOWN, LARGEST_SHOWN,
};
use crate::values::entry::FileSize;

/// One subtree's contribution, before it is folded into the parent's totals.
#[derive(Default)]
pub(super) struct Tally {
    size: u64,
    files: usize,
    directories: usize,
    /// Folders the walk could not list.
    pub(super) unread: usize,
    /// Entries named `.git`, files or folders, seen anywhere below.
    pub(super) git_entries: usize,
    by_kind: [(u64, usize); ContentKind::COUNT],
    largest: Vec<LargeFile>,
}

impl Tally {
    /// Bytes under the subtree walked so far.
    pub(super) const fn bytes(&self) -> u64 {
        self.size
    }

    pub(super) fn absorb(&mut self, other: Self) {
        self.size += other.size;
        self.files += other.files;
        self.directories += other.directories;
        self.unread += other.unread;
        self.git_entries += other.git_entries;
        for (dst, (size, files)) in self.by_kind.iter_mut().zip(other.by_kind) {
            dst.0 += size;
            dst.1 += files;
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

    /// Record a non-directory entry: regular files carry size and kind; a socket,
    /// device node or FIFO counts as one file with no size; links are skipped.
    pub(super) fn record_leaf(&mut self, name: &str, pfx: Option<&str>, leaf: (Shape, u64)) {
        if leaf.0 == Shape::File {
            record_file(self, name, pfx, leaf.1);
        } else if leaf.0 == Shape::Special {
            self.files += 1;
        }
    }
}

/// Trim a running list of large files back to what will be shown.
fn keep_largest(files: &mut Vec<LargeFile>) {
    files.sort_by_key(|file| std::cmp::Reverse(file.size));
    files.truncate(LARGEST_SHOWN);
}

/// The walk itself, with the budget passed in.
pub(super) fn analyse_within(directory: &Path, mut budget: Budget) -> DirectoryStats {
    let mut children = Vec::new();
    let mut total = Tally::default();
    let entries = match fs::read_dir(directory) {
        Ok(entries) => entries,
        Err(error) => return DirectoryStats::unreadable(&error),
    };
    for entry in entries.flatten() {
        if !budget.spend() {
            break;
        }
        // `metadata` on a `DirEntry` does not follow symlinks, which is what
        // keeps `a -> ..` from walking forever.
        let Ok(meta) = entry.metadata() else { continue };
        let name = entry.file_name();
        if meta.is_dir() {
            let mut name = name.to_string_lossy().into_owned();
            let mut sub = Tally::default();
            walk(&entry.path(), &mut name, &mut sub, &mut budget);
            children.push(sub.child(name));
            total.directories += 1;
            total.absorb(sub);
        } else {
            total.record_leaf(&name.to_string_lossy(), None, leaf(&meta));
        }
    }
    summarize(total, children, budget.stop())
}

pub(super) fn summarize(
    mut total: Tally,
    mut children: Vec<ChildSize>,
    stop: Option<Stop>,
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
        coverage: coverage(stop, total.unread),
    }
}

/// What a walk that ran into `stop` and left `unread` folders unlisted saw.
pub(super) const fn coverage(stop: Option<Stop>, unread: usize) -> Coverage {
    if stop.is_some() {
        Coverage::Partial
    } else if unread > 0 {
        Coverage::Unread(unread)
    } else {
        Coverage::Complete
    }
}

fn kind_totals(total: &Tally) -> Vec<KindTotal> {
    let mut by_kind: Vec<KindTotal> = ContentKind::ALL
        .into_iter()
        .zip(total.by_kind)
        .filter(|(_, (_, files))| *files > 0)
        .map(|(kind, (size, files))| KindTotal {
            kind,
            size: FileSize::from_bytes(size),
            files,
        })
        .collect();
    by_kind.sort_by(|a, b| b.size.cmp(&a.size).then_with(|| a.kind.cmp(&b.kind)));
    by_kind
}

/// Add one file to a tally, building its relative path only when it qualifies.
fn record_file(tally: &mut Tally, name: &str, prefix: Option<&str>, size: u64) {
    tally.size += size;
    tally.files += 1;
    if let Some(slot) = tally.by_kind.get_mut(ContentKind::of(name).index()) {
        slot.0 += size;
        slot.1 += 1;
    }
    let size = FileSize::from_bytes(size);
    if tally.largest.len() < LARGEST_SHOWN || tally.largest.last().is_none_or(|f| size > f.size) {
        let path = prefix.map_or_else(|| name.to_owned(), |dir| format!("{dir}/{name}"));
        tally.largest.push(LargeFile { path, size });
        keep_largest(&mut tally.largest);
    }
}

/// Walk one subtree, spending from the shared budget and reusing `prefix` across levels.
pub(super) fn walk(directory: &Path, prefix: &mut String, tally: &mut Tally, budget: &mut Budget) {
    let Ok(entries) = fs::read_dir(directory) else {
        tally.unread += 1;
        return;
    };
    for entry in entries.flatten() {
        if !budget.spend() {
            return;
        }
        let Ok(meta) = entry.metadata() else { continue };
        let raw_name = entry.file_name();
        let name = raw_name.to_string_lossy();
        if name == ".git" {
            tally.git_entries += 1;
        }

        if meta.is_dir() {
            tally.directories += 1;
            let base_len = prefix.len();
            prefix.push('/');
            prefix.push_str(&name);
            walk(&entry.path(), prefix, tally, budget);
            prefix.truncate(base_len);
        } else {
            tally.record_leaf(&name, Some(prefix.as_str()), leaf(&meta));
        }
    }
}
