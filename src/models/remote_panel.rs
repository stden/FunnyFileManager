//! Purpose: What a panel shows while it browses a remote host through a
//! stored session — the session, the directory, its entries, the cursor.
//!
//! State only: the listing is fetched by `ops::remote` through the
//! [`RemoteLink`] the panel carries, and the keys live in `popup::remote`.
//! Entries are the same [`DirEntry`] a local listing produces. The
//! connection lives here, with the panel, so that turning the panel back
//! into a file list drops it — a link kept elsewhere would outlive the panel
//! that used it.

use super::{remote_path, DirEntry, EntryKind, RemotePath, RowCursor, RowMotion, SessionName};
use crate::remote::RemoteLink;

mod marks;

/// The first row: up a directory, or out of the host at the root.
pub const PARENT: &str = "..";

/// The remote directory on screen and where the cursor is.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RemotePanel {
    /// The stored session the panel is connected through.
    session: SessionName,
    path: RemotePath,
    /// `..` first, then the listing.
    entries: Vec<DirEntry>,
    cursor: RowCursor,
    /// The first row drawn, kept between frames the way the file panel's is.
    pub scroll: usize,
    link: RemoteLink,
    marks: std::collections::BTreeSet<String>,
    previous_marks: std::collections::BTreeSet<String>,
}

/// What a listing turned into rows: how many entries were left out because
/// their names cannot be one part of a path.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[must_use]
pub struct Shown {
    pub unusable_names: usize,
}

impl RemotePanel {
    /// A panel on `session`'s root, through `link`, listing nothing yet.
    #[must_use]
    pub fn new(session: SessionName, link: RemoteLink) -> Self {
        Self {
            session,
            path: RemotePath::root(),
            entries: vec![parent_row()],
            cursor: RowCursor::TOP,
            scroll: 0,
            link,
            marks: std::collections::BTreeSet::new(),
            previous_marks: std::collections::BTreeSet::new(),
        }
    }

    /// The panel's title: the session and the directory, the way `NetBox`
    /// writes them.
    #[must_use]
    pub fn title(&self) -> String {
        format!("{}:{}", self.session, self.path)
    }

    #[must_use]
    pub const fn session(&self) -> &SessionName {
        &self.session
    }

    #[must_use]
    pub const fn path(&self) -> &RemotePath {
        &self.path
    }

    #[must_use]
    pub fn entries(&self) -> &[DirEntry] {
        &self.entries
    }

    #[must_use]
    pub const fn cursor(&self) -> usize {
        self.cursor.index()
    }

    #[must_use]
    pub const fn link(&self) -> &RemoteLink {
        &self.link
    }

    /// Show `listing` as the directory `path`, under the `..` row, with the
    /// cursor at the top. A name a host sent that cannot be a single path
    /// part — empty, `.`, `..`, or holding `/` — is left out and counted, so
    /// Enter can never walk the panel somewhere it did not show.
    pub fn show(&mut self, path: RemotePath, listing: Vec<DirEntry>) -> Shown {
        let selected = (path == self.path)
            .then(|| self.selected().map(|entry| entry.name.clone()))
            .flatten();
        if path != self.path {
            self.marks.clear();
            self.previous_marks.clear();
        }
        let total = listing.len();
        self.entries = std::iter::once(parent_row())
            .chain(
                listing
                    .into_iter()
                    .filter(|entry| remote_path::is_name(&entry.name)),
            )
            .collect();
        self.path = path;
        self.cursor = RowCursor::TOP;
        if let Some(index) =
            selected.and_then(|name| self.entries.iter().position(|entry| entry.name == name))
        {
            self.cursor = RowCursor::at(index, self.entries.len());
        }
        self.marks
            .retain(|name| self.entries.iter().any(|entry| entry.name == *name));
        self.scroll = 0;
        Shown {
            unusable_names: total.saturating_sub(self.entries.len().saturating_sub(1)),
        }
    }

    #[must_use]
    pub fn selected(&self) -> Option<&DirEntry> {
        self.entries.get(self.cursor.index())
    }

    /// Whether the cursor is on the `..` row.
    #[must_use]
    pub const fn on_parent(&self) -> bool {
        self.cursor.index() == 0
    }

    pub fn move_cursor(&mut self, motion: RowMotion) {
        self.cursor = self.cursor.moved(motion, self.entries.len());
    }

    /// A click on the `row`th row drawn: the cursor goes there. The row's
    /// index, or `None` below the last row.
    pub fn select_visible(&mut self, row: usize) -> Option<usize> {
        let index = self.scroll.saturating_add(row);
        (index < self.entries.len()).then(|| {
            self.cursor = RowCursor::at(index, self.entries.len());
            index
        })
    }
}

fn parent_row() -> DirEntry {
    DirEntry {
        name: PARENT.to_owned(),
        kind: EntryKind::Directory,
        ..DirEntry::default()
    }
}

#[cfg(test)]
mod tests;
