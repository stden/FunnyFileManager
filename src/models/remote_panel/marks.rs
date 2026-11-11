//! Purpose: Remote selection belongs to the remote listing, never the hidden local files.

use std::collections::BTreeSet;

use super::RemotePanel;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum UndoMarks {
    Snapshot(BTreeSet<String>),
    Toggled(String),
}

impl Default for UndoMarks {
    fn default() -> Self {
        Self::Snapshot(BTreeSet::new())
    }
}

impl RemotePanel {
    pub(crate) fn marked(&self, name: &str) -> bool {
        self.marks.contains(name)
    }

    pub(crate) fn action_names(&self) -> Vec<String> {
        if self.marks.is_empty() {
            return self
                .selected()
                .filter(|entry| entry.name != "..")
                .map(|entry| entry.name.clone())
                .into_iter()
                .collect();
        }
        self.entries
            .iter()
            .filter(|entry| self.marks.contains(&entry.name))
            .map(|entry| entry.name.clone())
            .collect()
    }

    pub(crate) fn toggle_mark(&mut self) {
        let Some(name) = self
            .selected()
            .filter(|entry| entry.name != "..")
            .map(|entry| entry.name.clone())
        else {
            return;
        };
        if !self.marks.remove(&name) {
            self.marks.insert(name.clone());
        }
        self.previous_marks = UndoMarks::Toggled(name);
        self.move_cursor(crate::models::RowMotion::Next);
    }

    pub(crate) fn invert_marks(&mut self) {
        let next = self
            .entries
            .iter()
            .filter(|entry| entry.name != ".." && !self.marks.contains(&entry.name))
            .map(|entry| entry.name.clone())
            .collect();
        let prev = std::mem::replace(&mut self.marks, next);
        self.previous_marks = UndoMarks::Snapshot(prev);
    }

    pub(crate) fn restore_marks(&mut self) {
        match &mut self.previous_marks {
            UndoMarks::Snapshot(prev) => std::mem::swap(&mut self.marks, prev),
            UndoMarks::Toggled(name) => {
                if !self.marks.remove(name) {
                    self.marks.insert(name.clone());
                }
            }
        }
    }

    pub(crate) fn mark_matching(&mut self, mask: &crate::mask::Mask, select: bool) {
        self.previous_marks = UndoMarks::Snapshot(self.marks.clone());
        for entry in self
            .entries
            .iter()
            .filter(|entry| entry.name != ".." && mask.matches(&entry.name))
        {
            if select {
                self.marks.insert(entry.name.clone());
            } else {
                self.marks.remove(&entry.name);
            }
        }
    }
}
