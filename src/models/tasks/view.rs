//! Purpose: Which processes the table shows and in what order — the sort, the
//! filter, the user scope and the kernel-thread switch — as a list of row
//! indexes into a snapshot, flat or as the parent tree.
//!
//! The view holds indexes, never copies of rows: a refresh replaces the
//! snapshot and the view is rebuilt from it in one pass.

mod filter;
mod list;
mod sort;
mod toggles;

use super::tree::{self, TreeLine};
use super::{ProcessKey, ProcessRow, ProcessSample};

pub use filter::{FilterTerms, UserScope};
pub use sort::{Sort, SortDirection, SortKey};
pub use toggles::{Arrangement, CommandView, KernelThreads};

/// Everything that decides which rows are shown and in what order.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ViewOptions {
    pub sort: Sort,
    pub arrangement: Arrangement,
    pub filter: FilterTerms,
    pub scope: UserScope,
    pub kernel_threads: KernelThreads,
}

impl ViewOptions {
    fn admits(&self, sample: &ProcessSample) -> bool {
        self.kernel_threads.admits(sample.kind)
            && self.scope.admits(&sample.owner)
            && self.filter.matches(sample)
    }
}

/// One shown row: which snapshot row it is, whose process, and its place in
/// the tree.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ViewRow {
    row: usize,
    key: ProcessKey,
    line: TreeLine,
}

impl ViewRow {
    /// `process`, row `row` of its snapshot, drawn as `line`.
    pub(super) const fn new(row: usize, process: &ProcessRow, line: TreeLine) -> Self {
        let key = process.sample().key;
        Self { row, key, line }
    }

    /// The index of the row in the snapshot it was built from.
    #[must_use]
    pub const fn row(self) -> usize {
        self.row
    }

    /// The process the row shows.
    #[must_use]
    pub const fn key(self) -> ProcessKey {
        self.key
    }

    #[must_use]
    pub const fn line(self) -> TreeLine {
        self.line
    }
}

/// The rows of `rows` that `options` shows, in the order it shows them.
#[must_use]
pub fn build_view(rows: &[ProcessRow], options: &ViewOptions) -> Vec<ViewRow> {
    let mut shown: Vec<(usize, &ProcessRow)> = rows
        .iter()
        .enumerate()
        .filter(|(_, process)| options.admits(process.sample()))
        .collect();
    shown.sort_unstable_by(|(_, left), (_, right)| options.sort.compare(left, right));
    match options.arrangement {
        Arrangement::Flat => shown
            .into_iter()
            .map(|(row, process)| ViewRow::new(row, process, TreeLine::Flat))
            .collect(),
        Arrangement::Tree => tree::arrange(&shown),
    }
}
