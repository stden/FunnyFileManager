//! Purpose: Cursor movement over the tree rows — keys, siblings, and the quick search that jumps by folder name.

use std::path::{Path, PathBuf};

use crate::casefold::starts_with_ignore_case;

use super::{Branch, Expansion, FolderTree, TreeRow};

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
        let Some(parent) = self
            .selected()
            .and_then(Path::parent)
            .map(Path::to_path_buf)
        else {
            return false;
        };
        self.move_to(|row| row.path == parent)
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
        let Some((depth, parent)) = self.level() else {
            return false;
        };
        let Some(offset) = self
            .rows
            .iter()
            .skip(self.cursor + 1)
            .position(|row| row.depth() == depth && row.path.parent() == parent.as_deref())
        else {
            return false;
        };
        self.cursor += offset + 1;
        true
    }

    /// Far's `Gray -`: the previous folder at the same level.
    pub fn previous_sibling(&mut self) -> bool {
        let Some((depth, parent)) = self.level() else {
            return false;
        };
        let Some(at) = self
            .rows
            .iter()
            .take(self.cursor)
            .rposition(|row| row.depth() == depth && row.path.parent() == parent.as_deref())
        else {
            return false;
        };
        self.cursor = at;
        true
    }

    fn level(&self) -> Option<(usize, Option<PathBuf>)> {
        let row = self.selected_row()?;
        Some((row.depth(), row.path.parent().map(Path::to_path_buf)))
    }

    fn move_to(&mut self, wanted: impl Fn(&TreeRow) -> bool) -> bool {
        let Some(at) = self.rows.iter().position(wanted) else {
            return false;
        };
        self.cursor = at;
        true
    }

    // ── quick search ───────────────────────────────────────────────

    /// The first folder whose name starts with `prefix`, from the top. The
    /// root is never a match: its "name" is a path, not something typed.
    pub fn find_first(&mut self, prefix: &str) -> bool {
        if prefix.is_empty() {
            return false;
        }
        self.move_to(|row| {
            row.branch != Branch::Root && starts_with_ignore_case(&row.name_cow(), prefix)
        })
    }

    /// The next match after the cursor, wrapping — Far's `Ctrl+Enter`.
    pub fn find_next(&mut self, prefix: &str) -> bool {
        if prefix.is_empty() || self.rows.is_empty() {
            return false;
        }
        let len = self.rows.len();
        let Some(offset) = (1..=len).find(|offset| {
            self.rows
                .get((self.cursor + offset) % len)
                .is_some_and(|row| {
                    row.branch != Branch::Root && starts_with_ignore_case(&row.name_cow(), prefix)
                })
        }) else {
            return false;
        };
        self.cursor = (self.cursor + offset) % len;
        true
    }
}
