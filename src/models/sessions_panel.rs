//! Purpose: What a panel shows when it has become `NetBox`'s stored-sessions
//! list — the rows, the cursor, and the `..` row that leads back to files.
//!
//! State only: the rows are built from the [`SessionStore`] by `ops`, and
//! the keys that move through them live in `popup`. The rows are a snapshot
//! rather than a reference into the store, so the renderer — which is handed
//! the panel alone — has what it needs; every change to the store rebuilds
//! them through `ops::sessions::refresh`.

use super::{RowCursor, RowMotion, SessionName};
use crate::sessions::SessionStore;

/// Far's title for the panel, with `NetBox`'s `[/]` marking the list's root.
pub const TITLE: &str = "NetBox: Stored sessions [/]";
/// The one column's heading.
pub const COLUMN_TITLE: &str = "Session name";
/// The row that returns the panel to files, spelled the way a listing spells
/// its parent.
pub const PARENT: &str = "..";

/// One line of the panel.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SessionRow {
    /// `..`: back to the file list.
    Parent,
    /// A stored session, with `user@host` when that is not already its name.
    Session {
        name: SessionName,
        login: Option<String>,
    },
}

impl SessionRow {
    #[must_use]
    pub const fn name(&self) -> Option<&SessionName> {
        match self {
            Self::Parent => None,
            Self::Session { name, .. } => Some(name),
        }
    }
}

/// The sessions on screen and where the cursor is.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionsPanel {
    /// Always begins with [`SessionRow::Parent`].
    rows: Vec<SessionRow>,
    cursor: RowCursor,
    /// The first row drawn, kept between frames the way the file panel's is.
    pub scroll: usize,
}

impl SessionsPanel {
    /// `..`, then a row for every stored session, sorted by name.
    #[must_use]
    pub fn from_store(store: &SessionStore) -> Self {
        let mut rows = vec![SessionRow::Parent];
        rows.extend(store.sorted().into_iter().map(|session| {
            let login = session.login();
            SessionRow::Session {
                login: (login != session.name.as_str()).then_some(login),
                name: session.name.clone(),
            }
        }));
        Self {
            rows,
            cursor: RowCursor::TOP,
            scroll: 0,
        }
    }

    /// Rebuild the rows from `store`, keeping the cursor on the same session
    /// when it is still there, and on the same row number when it is not —
    /// so deleting a session leaves the cursor on its neighbour.
    pub fn reload(&mut self, store: &SessionStore) {
        let kept = self.selected_name().cloned();
        let row = self.cursor.index();
        self.rows = Self::from_store(store).rows;
        let index = kept
            .and_then(|name| self.rows.iter().position(|row| row.name() == Some(&name)))
            .unwrap_or(row);
        self.cursor = RowCursor::at(index, self.rows.len());
    }

    #[must_use]
    pub fn rows(&self) -> &[SessionRow] {
        &self.rows
    }

    #[must_use]
    pub const fn cursor(&self) -> usize {
        self.cursor.index()
    }

    /// The session under the cursor, or `None` on `..`.
    #[must_use]
    pub fn selected_name(&self) -> Option<&SessionName> {
        self.rows
            .get(self.cursor.index())
            .and_then(SessionRow::name)
    }

    /// How many sessions are listed, the `..` row not counted.
    #[must_use]
    pub fn session_count(&self) -> usize {
        self.rows.len().saturating_sub(1)
    }

    /// Put the cursor on the session `name`, when it is listed.
    pub fn focus(&mut self, name: &SessionName) {
        if let Some(index) = self.rows.iter().position(|row| row.name() == Some(name)) {
            self.cursor = RowCursor::at(index, self.rows.len());
        }
    }

    pub fn move_cursor(&mut self, motion: RowMotion) {
        self.cursor = self.cursor.moved(motion, self.rows.len());
    }

    /// A click on the `row`th row drawn: the cursor goes there. The row's
    /// index, or `None` below the last row.
    pub fn select_visible(&mut self, row: usize) -> Option<usize> {
        let index = self.scroll.saturating_add(row);
        (index < self.rows.len()).then(|| {
            self.cursor = RowCursor::at(index, self.rows.len());
            index
        })
    }
}

#[cfg(test)]
mod tests;
