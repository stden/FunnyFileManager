//! Purpose: Finding a row of the task list: the search bar's query over the
//! last column, from the cursor and wrapping, and the jump to a PID.

use crate::casefold::contains_ignore_case;

use super::super::modal::Found;
use super::super::{Pid, TaskList};

/// Where a search starts looking.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SearchFrom {
    /// The cursor's own row first: every edit of the query.
    Cursor,
    /// The row after the cursor: `F3`, the next match.
    AfterCursor,
}

impl TaskList {
    /// Moves the cursor to the first row, `from` the cursor down and
    /// wrapping, whose last column contains `query`, ignoring case.
    pub fn find(&mut self, query: &str, from: SearchFrom, page: usize) -> Found {
        let len = self.view.len();
        let here = self.selection.cursor.map_or(0, |cursor| cursor.index);
        let first = match from {
            SearchFrom::Cursor => here,
            SearchFrom::AfterCursor => here.saturating_add(1),
        };
        let rows = self.rows();
        let found = (0..len).map(|step| (first + step) % len).find(|&at| {
            self.view
                .get(at)
                .and_then(|shown| rows.get(shown.row()))
                .is_some_and(|row| {
                    contains_ignore_case(self.command_view.text(row.sample()), query)
                })
        });
        self.select_found(found, page)
    }

    /// Moves the cursor to the shown process `pid`, if it is shown.
    pub fn jump_to_pid(&mut self, pid: Pid, page: usize) -> Found {
        let found = self.view.iter().position(|shown| shown.key().pid() == pid);
        self.select_found(found, page)
    }

    fn select_found(&mut self, found: Option<usize>, page: usize) -> Found {
        match found {
            Some(at) => {
                self.selection.place(&self.view, at, page);
                Found::Match
            }
            None => Found::NoMatch,
        }
    }
}
