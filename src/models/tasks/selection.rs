//! Purpose: The cursor, the scroll offset and the marks of the task list, and
//! every move over them: keys, clicks, search, PID jump and the marking keys.
//!
//! The cursor names a process by identity, not a row by number, so a refresh
//! or a re-sort leaves it on the process the user was looking at. Marks are
//! identities too, and are pruned when their process leaves the table.

mod marks;
mod search;

use std::collections::{BTreeSet, HashSet};

use crate::models::{RowCursor, RowMotion};
use crate::scroll::visible_window;

use super::view::ViewRow;
use super::{ProcessKey, ProcessRow, TaskList};

pub use search::SearchFrom;

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
    pub(crate) const fn with_page(self, page: usize) -> RowMotion {
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

/// Who put the cursor on its row.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
enum Placement {
    /// The list, on the first row as it opened. The first snapshot has no
    /// CPU% yet and is ordered by PID, so following that row's process took
    /// the cursor to PID 1 in the middle of the list once CPU% sorted it.
    #[default]
    Opened,
    /// The user, by a key, a click or a search: it follows its process.
    Chosen,
}

/// The cursor, the first shown row and the marks.
#[derive(Debug, Clone, Default)]
pub(super) struct Selection {
    cursor: Option<Cursor>,
    placement: Placement,
    offset: usize,
    marks: BTreeSet<ProcessKey>,
}

impl Selection {
    /// Finds the cursor's process in a rebuilt `view`. A process that left
    /// keeps the cursor's row, clamped to the list, and the cursor takes that
    /// row's process; so does a cursor nobody has moved yet. An empty list
    /// has no cursor.
    pub(super) fn follow(&mut self, view: &[ViewRow]) {
        let chosen = self.cursor.filter(|_| self.placement == Placement::Chosen);
        let found =
            chosen.and_then(|cursor| view.iter().position(|shown| shown.key() == cursor.key));
        let index = found
            .or_else(|| self.cursor.map(|cursor| cursor.index))
            .unwrap_or(0)
            .min(view.len().saturating_sub(1));
        self.cursor = cursor_at(view, index);
    }

    /// Puts the cursor on row `index` of `view` (clamped), and scrolls a body
    /// `page` rows tall just enough to show it.
    fn place(&mut self, view: &[ViewRow], index: usize, page: usize) {
        self.placement = Placement::Chosen;
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
}
