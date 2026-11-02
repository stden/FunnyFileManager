//! Purpose: Listing and selection totals, recomputed only after their inputs change.

use std::cell::RefCell;
use std::collections::HashMap;
use std::sync::Arc;

use super::Panel;
use crate::models::{DirEntry, FileSize};

type MarkedBucket = (usize, usize, u64);
type NameTotals = (Arc<()>, HashMap<String, MarkedBucket>);

#[derive(Debug, Clone)]
struct Cached<T> {
    entries: Arc<()>,
    marks: Arc<()>,
    value: T,
}

#[derive(Debug, Clone, Default)]
pub(super) struct PanelTotals {
    files: RefCell<Option<Cached<(usize, u64)>>>,
    marked: RefCell<Option<Cached<MarkedBucket>>>,
    by_name: RefCell<Option<NameTotals>>,
    #[cfg(test)]
    file_scans: std::cell::Cell<usize>,
    #[cfg(test)]
    mark_scans: std::cell::Cell<usize>,
    #[cfg(test)]
    index_builds: std::cell::Cell<usize>,
}

impl Panel {
    /// File count and bytes, excluding directories and the parent row.
    pub(crate) fn file_totals(&self) -> (usize, u64) {
        let mut cached = self.totals.files.borrow_mut();
        if let Some(saved) = cached
            .as_ref()
            .filter(|saved| Arc::ptr_eq(&saved.entries, self.entries.revision()))
        {
            return saved.value;
        }
        #[cfg(test)]
        self.totals.file_scans.set(self.totals.file_scans.get() + 1);
        let value = self
            .entries
            .iter()
            .filter(|entry| !entry.kind.is_dir())
            .fold((0, 0u64), |(count, bytes), entry| {
                (count + 1, bytes.saturating_add(entry.size.bytes()))
            });
        *cached = Some(Cached {
            entries: Arc::clone(self.entries.revision()),
            marks: Arc::clone(self.marked.revision()),
            value,
        });
        value
    }

    /// Selected folder count, file count and bytes, including measured folders.
    pub(crate) fn marked_totals(&self) -> MarkedBucket {
        let mut cached = self.totals.marked.borrow_mut();
        if let Some(saved) = cached.as_ref().filter(|saved| {
            Arc::ptr_eq(&saved.entries, self.entries.revision())
                && Arc::ptr_eq(&saved.marks, self.marked.revision())
        }) {
            return saved.value;
        }
        #[cfg(test)]
        self.totals.mark_scans.set(self.totals.mark_scans.get() + 1);
        let value = if self.marked.is_empty() {
            (0, 0, 0)
        } else {
            self.sum_marked_by_name()
        };
        *cached = Some(Cached {
            entries: Arc::clone(self.entries.revision()),
            marks: Arc::clone(self.marked.revision()),
            value,
        });
        value
    }

    fn sum_marked_by_name(&self) -> MarkedBucket {
        let mut guard = self.totals.by_name.borrow_mut();
        let stale = guard
            .as_ref()
            .is_none_or(|(rev, _)| !Arc::ptr_eq(rev, self.entries.revision()));
        if stale {
            #[cfg(test)]
            self.totals
                .index_builds
                .set(self.totals.index_builds.get() + 1);
            let mut map = HashMap::with_capacity(self.entries.len());
            for entry in &*self.entries {
                let bucket = map.entry(entry.name.clone()).or_insert((0, 0, 0));
                *bucket = add_marked(*bucket, entry);
            }
            *guard = Some((Arc::clone(self.entries.revision()), map));
        }
        let Some((_, map)) = guard.as_ref() else {
            return (0, 0, 0);
        };
        self.marked.iter().fold((0, 0, 0u64), |acc, name| {
            let Some(&(folders, files, bytes)) = map.get(name.as_str()) else {
                return acc;
            };
            (acc.0 + folders, acc.1 + files, acc.2.saturating_add(bytes))
        })
    }
}

fn add_marked((folders, files, bytes): MarkedBucket, entry: &DirEntry) -> MarkedBucket {
    if entry.kind.is_dir() {
        let added = entry.directory_size.measured().map_or(0, FileSize::bytes);
        (folders + 1, files, bytes.saturating_add(added))
    } else {
        (folders, files + 1, bytes.saturating_add(entry.size.bytes()))
    }
}

#[cfg(test)]
mod tests;
