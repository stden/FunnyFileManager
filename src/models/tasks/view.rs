//! Purpose: Which processes the table shows and in what order — the sort, the
//! filter, the user scope and the kernel-thread switch — as a list of row
//! indexes into a snapshot, flat or as the parent tree.
//!
//! The view holds indexes, never copies of rows: a refresh replaces the
//! snapshot and the view is rebuilt from it in one pass.

mod sort;

use std::collections::BTreeSet;

use crate::casefold::contains_ignore_case;
use crate::scroll::ScrollLeft;

use super::tree::{self, TreeLine};
use super::{Owner, ProcessKey, ProcessKind, ProcessRow, ProcessSample, TaskList};

pub use sort::{Sort, SortDirection, SortKey};

/// The table's layout: one row per process, or the parent tree.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum Arrangement {
    #[default]
    Flat,
    Tree,
}

impl Arrangement {
    /// The other layout: `F5`, `t`, `Ctrl+T`.
    #[must_use]
    pub const fn toggled(self) -> Self {
        match self {
            Self::Flat => Self::Tree,
            Self::Tree => Self::Flat,
        }
    }
}

/// What the last column shows: `F2` switches.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum CommandView {
    #[default]
    CommandLine,
    Name,
}

impl CommandView {
    #[must_use]
    pub const fn toggled(self) -> Self {
        match self {
            Self::CommandLine => Self::Name,
            Self::Name => Self::CommandLine,
        }
    }

    /// The text the last column shows for `sample`.
    #[must_use]
    pub fn text(self, sample: &ProcessSample) -> &str {
        match self {
            Self::CommandLine => &sample.command,
            Self::Name => &sample.name,
        }
    }
}

/// Whether kernel threads are listed: `K` switches. Hidden by default, since
/// on a busy host they bury the processes a user came to find.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum KernelThreads {
    Shown,
    #[default]
    Hidden,
}

impl KernelThreads {
    #[must_use]
    pub const fn toggled(self) -> Self {
        match self {
            Self::Shown => Self::Hidden,
            Self::Hidden => Self::Shown,
        }
    }

    const fn admits(self, kind: ProcessKind) -> bool {
        matches!(
            (self, kind),
            (Self::Shown, _) | (Self::Hidden, ProcessKind::User)
        )
    }
}

/// Whose processes are listed: `u` chooses.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub enum UserScope {
    #[default]
    All,
    Only(Owner),
}

impl UserScope {
    fn admits(&self, owner: &Owner) -> bool {
        match self {
            Self::All => true,
            Self::Only(only) => only == owner,
        }
    }
}

/// The live filter (`F4`): fixed strings separated by `|`, any of which a
/// process's name or command line must contain, ignoring case.
///
/// Keeps the line as typed beside the terms it parsed to, so the bar reopens
/// on what the user wrote and the footer names it.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct FilterTerms {
    line: String,
    /// Trimmed, lowercased and never empty.
    terms: Vec<String>,
}

impl FilterTerms {
    /// The filter `line` spells. Blank terms are dropped, so `fire||` is one
    /// term and a line of bars and blanks filters nothing.
    #[must_use]
    pub fn parse(line: &str) -> Self {
        let terms = line
            .split('|')
            .map(str::trim)
            .filter(|term| !term.is_empty())
            .map(str::to_lowercase);
        Self {
            line: line.to_owned(),
            terms: terms.collect(),
        }
    }

    /// The line as typed.
    #[must_use]
    pub fn line(&self) -> &str {
        &self.line
    }

    /// Whether the filter lets every process through.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.terms.is_empty()
    }

    /// Whether any term is part of `sample`'s name or command line.
    #[must_use]
    pub fn matches(&self, sample: &ProcessSample) -> bool {
        self.is_empty()
            || self.terms.iter().any(|term| {
                contains_ignore_case(&sample.name, term)
                    || contains_ignore_case(&sample.command, term)
            })
    }
}

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

impl TaskList {
    /// Everyone who owns a process in the snapshot, in order: the Show
    /// processes of menu.
    #[must_use]
    pub fn owners(&self) -> Vec<Owner> {
        let owners: BTreeSet<&Owner> = self.rows().iter().map(|row| &row.sample().owner).collect();
        owners.into_iter().cloned().collect()
    }

    /// Sorts by `key` — the reverse when it is already in force — and leaves
    /// the tree, as htop does when a sort key is chosen.
    pub fn choose_sort(&mut self, key: SortKey) {
        self.options.sort = self.options.sort.chosen(key);
        self.options.arrangement = Arrangement::Flat;
        self.rebuild();
    }

    /// `I`: the same key the other way round.
    pub fn invert_order(&mut self) {
        self.options.sort = self.options.sort.reversed();
        self.rebuild();
    }

    /// `F5`, `t`, `Ctrl+T`: the list or the tree.
    pub fn toggle_tree(&mut self) {
        self.options.arrangement = self.options.arrangement.toggled();
        self.rebuild();
    }

    /// `K`: kernel threads shown or hidden.
    pub fn toggle_kernel_threads(&mut self) {
        self.options.kernel_threads = self.options.kernel_threads.toggled();
        self.rebuild();
    }

    /// `u`, then a menu row: whose processes are shown.
    pub fn set_scope(&mut self, scope: UserScope) {
        self.options.scope = scope;
        self.rebuild();
    }

    /// Every edit of the filter bar: `line` parsed afresh.
    pub fn set_filter(&mut self, line: &str) {
        self.options.filter = FilterTerms::parse(line);
        self.rebuild();
    }

    /// `F2`: the command line or the name in the last column, scrolled back
    /// to its start.
    pub fn toggle_command_view(&mut self) {
        self.command_view = self.command_view.toggled();
        self.command_scroll = ScrollLeft::LEFT;
    }

    /// Left and Ctrl+Left: the last column `cells` back towards its start.
    pub fn pan_left(&mut self, cells: usize) {
        self.command_scroll = self.command_scroll.left(cells);
    }

    /// Right and Ctrl+Right: the last column `cells` further along.
    pub fn pan_right(&mut self, cells: usize) {
        self.command_scroll = self.command_scroll.right(cells);
    }
}
