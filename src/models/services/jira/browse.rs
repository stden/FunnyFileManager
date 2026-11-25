//! Purpose: Filtering, cursor movement, and multi-mark helpers for `JiraDialog`.

use super::JiraDialog;
use crate::models::{RowCursor, RowMotion};
use crate::values::jira::JiraIssue;

impl JiraDialog {
    #[must_use]
    pub(crate) fn filtered_indices(&self) -> Vec<usize> {
        let q = self.filter.trim().to_ascii_lowercase();
        self.issues
            .iter()
            .enumerate()
            .filter(|(_, issue)| {
                q.is_empty()
                    || [
                        issue.key.as_str(),
                        issue.summary.as_str(),
                        issue.status.as_str(),
                    ]
                    .iter()
                    .any(|s| s.to_ascii_lowercase().contains(&q))
            })
            .map(|(idx, _)| idx)
            .collect()
    }

    #[must_use]
    pub(crate) fn selected_issue(&self) -> Option<&JiraIssue> {
        let idx = *self.filtered_indices().get(self.cursor)?;
        self.issues.get(idx)
    }

    pub(crate) fn move_cursor(&mut self, motion: RowMotion) {
        let count = self.filtered_indices().len();
        self.cursor = RowCursor::at(self.cursor, count)
            .moved(motion, count)
            .index();
    }

    pub(crate) fn toggle_mark(&mut self) {
        if let Some(key) = self.selected_issue().map(|i| i.key.clone()) {
            if !self.marks.remove(&key) {
                self.marks.insert(key);
            }
        }
    }

    #[must_use]
    pub(crate) fn selected_or_marked(&self) -> Vec<JiraIssue> {
        if !self.marks.is_empty() {
            return self
                .issues
                .iter()
                .filter(|issue| self.marks.contains(&issue.key))
                .cloned()
                .collect();
        }
        self.selected_issue().cloned().into_iter().collect()
    }

    pub(crate) fn type_filter(&mut self, ch: char) {
        self.filter.push(ch);
        self.cursor = 0;
    }

    pub(crate) fn erase_filter(&mut self) {
        self.filter.pop();
        self.cursor = 0;
    }
}

#[cfg(test)]
mod tests;
