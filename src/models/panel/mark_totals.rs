//! Purpose: reuse the marked-byte totals while the marks and those sizes hold.
//!
//! The divider asks for the totals on every frame. Scanning the whole listing
//! each time costs O(n). The cache checks the marked names and the size of
//! each marked row in O(m), and scans only when that check fails.

use std::{
    collections::BTreeSet,
    hash::{Hash, Hasher},
};

use super::Panel;
use crate::models::{DirEntry, FileName, FileSize};

#[derive(Debug, Clone)]
pub(super) struct MarkCache {
    ptr: usize,
    len: usize,
    names: u64,
    rows: Vec<MarkedRow>,
    totals: (usize, usize, u64),
}

#[derive(Debug, Clone)]
struct MarkedRow {
    index: usize,
    name: String,
    bytes: u64,
    folder: bool,
}

impl Panel {
    /// Folders, files and bytes among the marked names that are listed.
    ///
    /// `None` when nothing marked is listed, which is also the answer for an
    /// empty mark set.
    pub(crate) fn marked_totals(&self) -> Option<(usize, usize, u64)> {
        if self.marked.is_empty() {
            return None;
        }
        if let Some(cache) = self.mark_cache.borrow().as_ref() {
            if let Cached::Hit(totals) = cache.if_fresh(self) {
                return totals;
            }
        }
        let cache = MarkCache::collect(self);
        let totals = nonempty(cache.totals);
        *self.mark_cache.borrow_mut() = Some(cache);
        totals
    }
}

enum Cached {
    Miss,
    Hit(Option<(usize, usize, u64)>),
}

impl MarkCache {
    fn if_fresh(&self, panel: &Panel) -> Cached {
        if self.matches(panel) {
            Cached::Hit(nonempty(self.totals))
        } else {
            Cached::Miss
        }
    }

    fn matches(&self, panel: &Panel) -> bool {
        self.ptr == panel.entries.as_ptr().addr()
            && self.len == panel.entries.len()
            && self.names == hash_marked(&panel.marked)
            && self.rows.iter().all(|row| row.matches(panel))
    }

    fn collect(panel: &Panel) -> Self {
        let mut totals = (0usize, 0usize, 0u64);
        let mut rows = Vec::new();
        for (index, entry) in panel.entries.iter().enumerate() {
            if panel.marked.contains(entry.name.as_str()) {
                push_marked(&mut rows, &mut totals, index, entry);
            }
        }
        Self {
            ptr: panel.entries.as_ptr().addr(),
            len: panel.entries.len(),
            names: hash_marked(&panel.marked),
            rows,
            totals,
        }
    }
}

fn push_marked(
    rows: &mut Vec<MarkedRow>,
    totals: &mut (usize, usize, u64),
    index: usize,
    entry: &DirEntry,
) {
    let folder = entry.kind.is_dir();
    let size = marked_bytes(entry);
    if folder {
        totals.0 += 1;
    } else {
        totals.1 += 1;
    }
    totals.2 = totals.2.saturating_add(size);
    rows.push(MarkedRow {
        index,
        name: entry.name.clone(),
        bytes: size,
        folder,
    });
}

impl MarkedRow {
    fn matches(&self, panel: &Panel) -> bool {
        panel.entries.get(self.index).is_some_and(|entry| {
            entry.name == self.name
                && entry.kind.is_dir() == self.folder
                && marked_bytes(entry) == self.bytes
        })
    }
}

fn marked_bytes(entry: &DirEntry) -> u64 {
    if entry.kind.is_dir() {
        entry.directory_size.measured().map_or(0, FileSize::bytes)
    } else {
        entry.size.bytes()
    }
}

fn nonempty(totals: (usize, usize, u64)) -> Option<(usize, usize, u64)> {
    let (folders, files, _) = totals;
    (folders + files > 0).then_some(totals)
}

fn hash_marked(marked: &BTreeSet<FileName>) -> u64 {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    marked.len().hash(&mut hasher);
    for name in marked {
        name.as_str().hash(&mut hasher);
    }
    hasher.finish()
}
