//! Purpose: Remote selection belongs to the remote listing, never the hidden local files.

use super::RemotePanel;

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
        self.previous_marks.clone_from(&self.marks);
        if !self.marks.remove(&name) {
            self.marks.insert(name);
        }
        self.move_cursor(crate::models::RowMotion::Next);
    }

    pub(crate) fn invert_marks(&mut self) {
        self.previous_marks.clone_from(&self.marks);
        self.marks = self
            .entries
            .iter()
            .filter(|entry| entry.name != ".." && !self.marks.contains(&entry.name))
            .map(|entry| entry.name.clone())
            .collect();
    }

    pub(crate) fn restore_marks(&mut self) {
        std::mem::swap(&mut self.marks, &mut self.previous_marks);
    }

    pub(crate) fn mark_matching(&mut self, mask: &crate::mask::Mask, select: bool) {
        self.previous_marks.clone_from(&self.marks);
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
