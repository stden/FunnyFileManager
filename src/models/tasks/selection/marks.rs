//! Purpose: The marks of the task list — which processes are marked and how
//! many are shown — and the marking keys: Space and Insert, `c`, `U`, `*`, and
//! gray `+` and `-`.

use crate::mask::Mask;
use crate::models::MaskPurpose;

use super::super::notice::Notice;
use super::super::tree::descendants;
use super::super::{ProcessKey, TaskList};
use super::ListMotion;

impl TaskList {
    #[must_use]
    pub fn is_marked(&self, key: ProcessKey) -> bool {
        self.selection.marks.contains(&key)
    }

    /// Marked processes among the shown rows: what a kill would target.
    #[must_use]
    pub fn marked_count(&self) -> usize {
        self.view
            .iter()
            .filter(|shown| self.is_marked(shown.key()))
            .count()
    }

    /// `Space` and `Insert`: marks or unmarks the cursor's process, then
    /// moves down.
    pub fn toggle_mark(&mut self, page: usize) {
        if let Some(cursor) = self.selection.cursor {
            let marks = &mut self.selection.marks;
            if !marks.remove(&cursor.key) {
                marks.insert(cursor.key);
            }
        }
        self.move_cursor(ListMotion::Down, page);
    }

    /// `c`: marks the cursor's process and everything descending from it in
    /// the snapshot.
    pub fn mark_family(&mut self) {
        let Some(cursor) = self.selection.cursor else {
            return;
        };
        let Some(shown) = self.view.get(cursor.index) else {
            return;
        };
        let rows = self.rows();
        let family = descendants(rows, shown.row()).into_iter();
        let keys: Vec<ProcessKey> = family
            .filter_map(|at| rows.get(at))
            .map(|row| row.sample().key)
            .collect();
        self.selection.marks.extend(keys);
    }

    /// `U`: unmarks everything, shown or not.
    pub fn unmark_all(&mut self) {
        self.selection.marks.clear();
    }

    /// `*`: inverts the marks of the shown rows.
    pub fn invert_marks(&mut self) {
        for shown in &self.view {
            let marks = &mut self.selection.marks;
            if !marks.remove(&shown.key()) {
                marks.insert(shown.key());
            }
        }
    }

    /// Gray `+` and `-`: marks or unmarks the shown processes whose name
    /// `mask` matches, and says how many changed.
    pub fn mark_by_mask(&mut self, purpose: MaskPurpose, mask: &Mask) -> Notice {
        let rows = self.table.rows();
        let marks = &mut self.selection.marks;
        let changed = self
            .view
            .iter()
            .filter(|shown| {
                rows.get(shown.row())
                    .is_some_and(|row| mask.matches(&row.sample().name))
            })
            .filter(|shown| match purpose {
                MaskPurpose::Select => marks.insert(shown.key()),
                MaskPurpose::Unselect => marks.remove(&shown.key()),
            })
            .count();
        Notice::Marked {
            changed,
            total: self.marked_count(),
        }
    }
}
