//! Purpose: Cursor movement over the tree rows — keys, siblings, and the quick search that jumps by folder name.

use std::cell::RefCell;

use crate::casefold::{folded_name, PrefixIndex};

use super::{Branch, Expansion, FolderTree, TreeRow};

#[derive(Debug, Default)]
pub(super) struct TreeSearchCache(RefCell<Option<PrefixIndex>>);

impl Clone for TreeSearchCache {
    fn clone(&self) -> Self {
        Self::default()
    }
}

impl PartialEq for TreeSearchCache {
    fn eq(&self, _other: &Self) -> bool {
        true
    }
}

impl Eq for TreeSearchCache {}

impl TreeSearchCache {
    pub(super) fn clear(&mut self) {
        self.0 = RefCell::new(None);
    }

    fn with<R>(&self, rows: &[TreeRow], f: impl FnOnce(&PrefixIndex) -> R) -> R {
        let mut guard = self.0.borrow_mut();
        let index = guard.get_or_insert_with(|| build_search_index(rows));
        f(index)
    }
}

fn build_search_index(rows: &[TreeRow]) -> PrefixIndex {
    let entries = rows
        .iter()
        .enumerate()
        .filter(|(_, row)| row.branch != Branch::Root)
        .map(|(idx, row)| (folded_name(&row.name_cow()), idx))
        .collect();
    PrefixIndex::new(entries)
}

impl FolderTree {
    // ── cursor ─────────────────────────────────────────────────────

    pub fn up(&mut self) {
        self.cursor = self.cursor.saturating_sub(1);
    }

    pub fn down(&mut self) {
        self.cursor = (self.cursor + 1).min(self.rows.len().saturating_sub(1));
    }

    pub fn first(&mut self) {
        self.cursor = 0;
    }

    pub fn last(&mut self) {
        self.cursor = self.rows.len().saturating_sub(1);
    }

    pub fn page_up(&mut self, rows: usize) {
        self.cursor = self.cursor.saturating_sub(rows);
    }

    pub fn page_down(&mut self, rows: usize) {
        self.cursor = self
            .cursor
            .saturating_add(rows)
            .min(self.rows.len().saturating_sub(1));
    }

    /// Up to the row this folder hangs off. `false` on the root.
    pub fn move_to_parent(&mut self) -> bool {
        self.jump_to(self.links.get(self.cursor).and_then(|link| link.parent))
    }

    /// Down to the first subfolder on screen. `false` unless the row is expanded.
    pub fn move_to_first_child(&mut self) -> bool {
        let Some(row) = self.selected_row() else {
            return false;
        };
        if row.expansion != Expansion::Expanded {
            return false;
        }
        self.cursor += 1;
        true
    }

    /// Far's `Gray +`: the next folder at the same level under the same
    /// parent, skipping whatever is open in between.
    pub fn next_sibling(&mut self) -> bool {
        self.jump_to(
            self.links
                .get(self.cursor)
                .and_then(|link| link.next_sibling),
        )
    }

    /// Far's `Gray -`: the previous folder at the same level.
    pub fn previous_sibling(&mut self) -> bool {
        self.jump_to(
            self.links
                .get(self.cursor)
                .and_then(|link| link.prev_sibling),
        )
    }

    fn jump_to(&mut self, target: Option<usize>) -> bool {
        let Some(row) = target else {
            return false;
        };
        self.cursor = row;
        true
    }

    // ── quick search ───────────────────────────────────────────────

    /// The first folder whose name starts with `prefix`, from the top. The
    /// root is never a match: its "name" is a path, not something typed.
    pub fn find_first(&mut self, prefix: &str) -> bool {
        if prefix.is_empty() || self.rows.is_empty() {
            return false;
        }
        let target = self
            .search_cache
            .with(&self.rows, |index| index.choose(prefix, 0, false));
        self.jump_to(target)
    }

    /// The next match after the cursor, wrapping — Far's `Ctrl+Enter`.
    pub fn find_next(&mut self, prefix: &str) -> bool {
        if prefix.is_empty() || self.rows.is_empty() {
            return false;
        }
        let current = self.cursor;
        let target = self
            .search_cache
            .with(&self.rows, |index| index.choose(prefix, current, true));
        self.jump_to(target)
    }
}
