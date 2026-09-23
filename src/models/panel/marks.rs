//! Purpose: marks entries for file operations and remembers the previous selection.

use std::collections::BTreeSet;

use crate::models::FileName;

use super::Panel;

impl Panel {
    pub fn toggle_mark_selected(&mut self) -> bool {
        let Some(selected_name) = self.get_selected_entry().map(|entry| entry.name.clone()) else {
            return false;
        };

        if selected_name == ".." {
            return false;
        }

        if self.marked.contains(selected_name.as_str()) {
            self.marked.remove(selected_name.as_str());
        } else {
            self.marked.insert(FileName::new(selected_name));
        }

        true
    }

    pub fn clear_marks(&mut self) {
        self.remember_marks();
        self.marked.clear();
    }

    /// Keep the current selection for Ctrl+M, before replacing it.
    pub(crate) fn remember_marks(&mut self) {
        self.previous_marks.clone_from(&self.marked);
    }

    /// Every entry this panel can act on — the listing without its `..` row,
    /// which is not a file and must never end up marked.
    fn markable(&self) -> impl Iterator<Item = &str> {
        self.entries
            .iter()
            .map(|entry| entry.name.as_str())
            .filter(|name| *name != "..")
    }

    /// Gray + / Gray −: add every entry matching `mask` to the selection, or
    /// take every one of them out of it. Returns how many rows changed, which
    /// is what the status line reports.
    pub fn mark_matching(&mut self, mask: &crate::mask::Mask, select: bool) -> usize {
        self.remember_marks();
        // Collected before anything is marked: the scan borrows the listing,
        // and the marking mutates the set sitting beside it.
        let hits: Vec<FileName> = self
            .markable()
            .filter(|name| mask.matches(name))
            .map(|name| FileName::new(name.to_owned()))
            .collect();
        let mut changed = 0;
        for name in hits {
            let moved = if select {
                self.marked.insert(name)
            } else {
                self.marked.remove(name.as_str())
            };
            changed += usize::from(moved);
        }
        changed
    }

    /// `:mark all`: every entry joins the selection. Recorded first, like any
    /// other bulk change, so Ctrl+M can take it back.
    pub fn mark_all(&mut self) {
        self.remember_marks();
        let all: Vec<FileName> = self.markable().map(FileName::from).collect();
        self.marked.extend(all);
    }

    /// Gray ∗: everything selected becomes unselected and the other way round.
    pub fn invert_marks(&mut self) {
        self.remember_marks();
        let inverted: BTreeSet<FileName> = self
            .markable()
            .filter(|name| !self.marked.contains(*name))
            .map(|name| FileName::new(name.to_owned()))
            .collect();
        self.marked = inverted;
    }

    /// Ctrl+M: put back the selection as it was before the last change.
    ///
    /// A swap rather than an assignment, so pressing it twice returns to where
    /// it started — the undo of an undo, which is what a user who pressed it
    /// by mistake expects.
    pub fn restore_marks(&mut self) {
        std::mem::swap(&mut self.marked, &mut self.previous_marks);
    }

    #[allow(dead_code)]
    pub fn action_entry_names(&self) -> Vec<String> {
        if !self.marked.is_empty() {
            return self
                .marked
                .iter()
                .map(FileName::as_str)
                .map(str::to_owned)
                .collect();
        }

        self.get_selected_entry()
            .and_then(|entry| {
                if entry.name == ".." {
                    None
                } else {
                    Some(entry.name.clone())
                }
            })
            .into_iter()
            .collect()
    }
}
