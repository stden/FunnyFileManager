//! Purpose: The cursor, the scroll offset and the marks of the task list, and
//! every move over them: keys, clicks, search, PID jump and the marking keys.
//!
//! The cursor names a process by identity, not a row by number, so a refresh
//! or a re-sort leaves it on the process the user was looking at. Marks are
//! identities too, and are pruned when their process leaves the table.

use std::collections::{BTreeSet, HashSet};

use crate::casefold::contains_ignore_case;
use crate::mask::Mask;
use crate::models::{MaskPurpose, RowCursor, RowMotion};
use crate::scroll::visible_window;

use super::modal::Found;
use super::notice::Notice;
use super::tree::descendants;
use super::view::ViewRow;
use super::{Pid, ProcessKey, ProcessRow, TaskList};

/// Where the cursor is: the process, and the row it was last seen on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Cursor {
    key: ProcessKey,
    index: usize,
}

impl Cursor {
    #[must_use]
    pub const fn key(self) -> ProcessKey {
        self.key
    }

    /// The row in the view, counted from the top of the list.
    #[must_use]
    pub const fn index(self) -> usize {
        self.index
    }
}

/// A cursor move by key or wheel. Unlike [`RowMotion`] it carries no page:
/// a key is resolved before the page is known.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ListMotion {
    Up,
    Down,
    PageUp,
    PageDown,
    Home,
    End,
}

impl ListMotion {
    /// The move over a list whose page is `page` rows.
    const fn with_page(self, page: usize) -> RowMotion {
        match self {
            Self::Up => RowMotion::Previous,
            Self::Down => RowMotion::Next,
            Self::PageUp => RowMotion::PageUp(page),
            Self::PageDown => RowMotion::PageDown(page),
            Self::Home => RowMotion::First,
            Self::End => RowMotion::Last,
        }
    }
}

/// Where a search starts looking.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SearchFrom {
    /// The cursor's own row first: every edit of the query.
    Cursor,
    /// The row after the cursor: `F3`, the next match.
    AfterCursor,
}

/// The cursor, the first shown row and the marks.
#[derive(Debug, Clone, Default)]
pub(super) struct Selection {
    cursor: Option<Cursor>,
    offset: usize,
    marks: BTreeSet<ProcessKey>,
}

impl Selection {
    /// Finds the cursor's process in a rebuilt `view`. A process that left
    /// keeps the cursor's row, clamped to the list, and the cursor takes that
    /// row's process; an empty list has no cursor.
    pub(super) fn follow(&mut self, view: &[ViewRow]) {
        let found = self
            .cursor
            .and_then(|cursor| view.iter().position(|shown| shown.key() == cursor.key));
        let index = found
            .or_else(|| self.cursor.map(|cursor| cursor.index))
            .unwrap_or(0)
            .min(view.len().saturating_sub(1));
        self.cursor = cursor_at(view, index);
    }

    /// Puts the cursor on row `index` of `view` (clamped), and scrolls a body
    /// `page` rows tall just enough to show it.
    fn place(&mut self, view: &[ViewRow], index: usize, page: usize) {
        self.cursor = cursor_at(view, index.min(view.len().saturating_sub(1)));
        self.clamp(view.len(), page);
    }

    /// Keeps the offset showing the cursor on a body `page` rows tall.
    pub(super) fn clamp(&mut self, len: usize, page: usize) {
        let index = self.cursor.map(|cursor| cursor.index);
        self.offset = visible_window(len, page, self.offset, index).start;
    }

    /// Drops the marks of processes no longer in `rows`.
    pub(super) fn prune(&mut self, rows: &[ProcessRow]) {
        if self.marks.is_empty() {
            return;
        }
        let present: HashSet<ProcessKey> = rows.iter().map(|row| row.sample().key).collect();
        self.marks.retain(|key| present.contains(key));
    }
}

fn cursor_at(view: &[ViewRow], index: usize) -> Option<Cursor> {
    view.get(index).map(|shown| Cursor {
        key: shown.key(),
        index,
    })
}

/// The row `motion` moves to from `from` in a list `len` long.
fn target(from: usize, motion: ListMotion, len: usize, page: usize) -> usize {
    RowCursor::at(from, len)
        .moved(motion.with_page(page), len)
        .index()
}

impl TaskList {
    #[must_use]
    pub const fn cursor(&self) -> Option<Cursor> {
        self.selection.cursor
    }

    /// The first row a body shows, before the renderer fits it to the
    /// cursor with `visible_window`.
    #[must_use]
    pub const fn offset(&self) -> usize {
        self.selection.offset
    }

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

    /// Moves the cursor on a body `page` rows tall.
    pub fn move_cursor(&mut self, motion: ListMotion, page: usize) {
        let from = self.selection.cursor.map_or(0, |cursor| cursor.index);
        let to = target(from, motion, self.view.len(), page);
        self.selection.place(&self.view, to, page);
    }

    /// Puts the cursor on view row `index`: a click on the table.
    pub fn select_index(&mut self, index: usize, page: usize) {
        self.selection.place(&self.view, index, page);
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
