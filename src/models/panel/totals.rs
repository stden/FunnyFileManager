//! Purpose: Listing and selection totals, recomputed only after their inputs change.

use std::cell::{Cell, RefCell};

use super::collection::Revision;
use super::Panel;
use crate::models::{DirEntry, FileSize};

mod index;
use index::{MarkedCounts, NameIndex};

#[derive(Debug, Clone, Copy)]
struct Cached<T> {
    entries: Revision,
    marks: Revision,
    value: T,
}

#[derive(Debug, Clone, Copy)]
struct PendingMarkDelta {
    entries: Revision,
    from_marks: Revision,
    to_marks: Revision,
    entry_idx: usize,
    added: bool,
}

#[derive(Debug, Clone, Default)]
pub(super) struct PanelTotals {
    files: Cell<Option<Cached<(usize, u64)>>>,
    marked: Cell<Option<Cached<MarkedCounts>>>,
    by_name: RefCell<Option<NameIndex>>,
    pending_delta: Cell<Option<PendingMarkDelta>>,
    #[cfg(test)]
    file_scans: Cell<usize>,
    #[cfg(test)]
    mark_scans: Cell<usize>,
    #[cfg(test)]
    index_builds: Cell<usize>,
}

impl PanelTotals {
    pub(super) fn record_mark_toggle(
        &self,
        entries: Revision,
        from_marks: Revision,
        to_marks: Revision,
        entry_idx: usize,
        added: bool,
    ) {
        self.pending_delta.set(Some(PendingMarkDelta {
            entries,
            from_marks,
            to_marks,
            entry_idx,
            added,
        }));
    }
}

impl Panel {
    /// File count and bytes, excluding directories and the parent row.
    pub(crate) fn file_totals(&self) -> (usize, u64) {
        let entries_rev = self.entries.rev();
        if let Some(saved) = self.totals.files.get().filter(|s| s.entries == entries_rev) {
            return saved.value;
        }
        #[cfg(test)]
        self.totals.file_scans.set(self.totals.file_scans.get() + 1);
        let value = self
            .entries
            .iter()
            .filter(|entry| !entry.kind.is_dir())
            .fold((0, 0_u64), |(count, bytes), entry| {
                (count + 1, bytes.saturating_add(entry.size.bytes()))
            });
        self.totals.files.set(Some(Cached {
            entries: entries_rev,
            marks: self.marked.rev(),
            value,
        }));
        value
    }

    /// Selected folder count, file count and bytes, including measured folders.
    pub(crate) fn marked_totals(&self) -> (usize, usize, u64) {
        let entries_rev = self.entries.rev();
        let marks_rev = self.marked.rev();
        let prev = self.totals.marked.get();
        if let Some(saved) = prev.filter(|s| s.entries == entries_rev && s.marks == marks_rev) {
            return saved.value.into_tuple();
        }
        #[cfg(test)]
        self.totals.mark_scans.set(self.totals.mark_scans.get() + 1);
        let delta = self.totals.pending_delta.take();
        let value = if self.marked.is_empty() {
            MarkedCounts::default()
        } else if let Some(updated) = prev.and_then(|s| self.apply_mark_delta(s, delta)) {
            updated
        } else {
            self.sum_marked_by_name()
        };
        self.totals.marked.set(Some(Cached {
            entries: entries_rev,
            marks: marks_rev,
            value,
        }));
        value.into_tuple()
    }

    fn apply_mark_delta(
        &self,
        saved: Cached<MarkedCounts>,
        delta: Option<PendingMarkDelta>,
    ) -> Option<MarkedCounts> {
        let d = delta?;
        // Once `bytes` saturates at `u64::MAX`, unmarking cannot subtract cleanly and must rescan.
        if saved.entries != self.entries.rev()
            || d.entries != saved.entries
            || d.from_marks != saved.marks
            || d.to_marks != self.marked.rev()
            || (!d.added && saved.value.bytes == u64::MAX)
        {
            return None;
        }
        let name = &self.entries.get(d.entry_idx)?.name;
        let counts = self.with_name_index(|idx| idx.get(&self.entries, name))?;
        Some(step_counts(saved.value, counts, d.added))
    }

    fn with_name_index<R>(&self, f: impl FnOnce(&NameIndex) -> R) -> R {
        let entries_rev = self.entries.rev();
        let mut guard = self.totals.by_name.borrow_mut();
        let index = match guard.as_ref() {
            Some(existing) if existing.entries == entries_rev => existing,
            _ => {
                #[cfg(test)]
                self.totals
                    .index_builds
                    .set(self.totals.index_builds.get() + 1);
                guard.insert(NameIndex::build(entries_rev, &self.entries))
            }
        };
        f(index)
    }

    fn sum_marked_by_name(&self) -> MarkedCounts {
        self.with_name_index(|index| {
            self.marked
                .iter()
                .fold(MarkedCounts::default(), |acc, name| {
                    let counts = index.get(&self.entries, name.as_str()).unwrap_or_default();
                    step_counts(acc, counts, true)
                })
        })
    }
}

fn step_counts(cur: MarkedCounts, delta: MarkedCounts, added: bool) -> MarkedCounts {
    if added {
        MarkedCounts {
            folders: cur.folders + delta.folders,
            files: cur.files + delta.files,
            bytes: cur.bytes.saturating_add(delta.bytes),
        }
    } else {
        MarkedCounts {
            folders: cur.folders.saturating_sub(delta.folders),
            files: cur.files.saturating_sub(delta.files),
            bytes: cur.bytes.saturating_sub(delta.bytes),
        }
    }
}

fn add_marked(counts: MarkedCounts, entry: &DirEntry) -> MarkedCounts {
    if entry.kind.is_dir() {
        let added = entry.directory_size.measured().map_or(0, FileSize::bytes);
        MarkedCounts {
            folders: counts.folders + 1,
            files: counts.files,
            bytes: counts.bytes.saturating_add(added),
        }
    } else {
        MarkedCounts {
            folders: counts.folders,
            files: counts.files + 1,
            bytes: counts.bytes.saturating_add(entry.size.bytes()),
        }
    }
}

#[cfg(test)]
mod tests;
