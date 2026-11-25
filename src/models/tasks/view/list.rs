//! Purpose: The task list's view changes — sort, reverse, tree, kernel
//! threads, user scope and filter, each rebuilding the shown rows — and the
//! last column's switch and sideways scroll.

use std::collections::BTreeSet;

use crate::scroll::ScrollLeft;

use super::super::{Owner, TaskList};
use super::{Arrangement, FilterTerms, SortKey, UserScope};

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
