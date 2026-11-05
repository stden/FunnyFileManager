//! Purpose: The database explorer's state and the transitions of its keys.
//!
//! The cursor in the tree, the view over it and the one request in flight. A key
//! that needs data queues a request on the caller's `Requests`; the answer comes
//! back through `receive`, and an answer nobody waits for is dropped.

pub mod load;
pub mod nav;
pub mod nodes;
pub mod tree;
pub mod view;

use super::ConnectionSpec;
pub(crate) use tree::{Connection, Expansion, Load, Row, TableNode, TablePath};
pub(crate) use view::{Back, RowsView, View};
use view::{Pending, Want};

#[cfg(test)]
mod tests;

/// The explorer dialog's state. Public only because `Overlay` names it; every
/// method is crate-internal.
#[derive(Debug, Clone)]
pub struct DatabaseExplorer {
    connections: Vec<Connection>,
    cursor: usize,
    view: View,
    pending: Option<Pending>,
    notice: Option<String>,
}

impl DatabaseExplorer {
    /// An explorer over `specs`; `notice` is the hint shown until a request
    /// replaces it, for example why there are no connections.
    pub(crate) fn new(specs: Vec<ConnectionSpec>, notice: Option<String>) -> Self {
        Self {
            connections: specs.into_iter().map(Connection::new).collect(),
            cursor: 0,
            view: View::Tree,
            pending: None,
            notice,
        }
    }

    pub(crate) fn connections(&self) -> &[Connection] {
        &self.connections
    }

    pub(crate) fn rows(&self) -> Vec<Row> {
        tree::visible(&self.connections)
    }

    pub(crate) const fn cursor(&self) -> usize {
        self.cursor
    }

    pub(crate) const fn view(&self) -> &View {
        &self.view
    }

    pub(crate) fn notice(&self) -> Option<&str> {
        self.notice.as_deref()
    }

    pub(crate) const fn is_loading(&self) -> bool {
        self.pending.is_some()
    }

    /// Whether the request in flight is the one that fills `row`.
    pub(crate) fn is_loading_row(&self, row: Row) -> bool {
        let Some(pending) = &self.pending else {
            return false;
        };
        match (row, pending.want) {
            (Row::Connection(connection), Want::Schemas(wanted)) => connection == wanted,
            (Row::Table(path), Want::Columns(wanted)) => path == wanted,
            _ => false,
        }
    }

    pub(crate) fn table(&self, path: TablePath) -> Option<&TableNode> {
        tree::table_at(&self.connections, path)
    }

    /// Whether the row is expanded; a column never is.
    pub(crate) fn is_open(&self, row: Row) -> bool {
        tree::is_open(&self.connections, row)
    }

    /// The name a row shows: a connection's, a schema's, a table's or a column's.
    pub(crate) fn name(&self, row: Row) -> Option<&str> {
        match row {
            Row::Connection(connection) => self
                .connections
                .get(connection)
                .map(|node| node.spec.name.as_str()),
            Row::Schema(connection, schema) => {
                tree::schema_at(&self.connections, connection, schema)
                    .map(|node| node.name.as_str())
            }
            Row::Table(path) => self.table(path).map(|node| node.name.as_str()),
            Row::Column(path, index) => self
                .table(path)?
                .columns
                .ready()?
                .get(index)
                .map(|column| column.name.as_str()),
        }
    }

    /// Esc: from a view back to the tree; from the tree, close the dialog.
    pub(crate) fn back(&mut self) -> Back {
        if matches!(self.view, View::Tree) {
            return Back::Close;
        }
        self.view = View::Tree;
        Back::ToTree
    }

    fn current(&self) -> Option<Row> {
        self.rows().get(self.cursor).copied()
    }

    fn clamp_cursor(&mut self) {
        let last = self.rows().len().saturating_sub(1);
        self.cursor = self.cursor.min(last);
    }

    fn position_of(&self, row: Row) -> usize {
        self.rows()
            .iter()
            .position(|candidate| *candidate == row)
            .unwrap_or(self.cursor)
    }

    fn set_expansion(&mut self, row: Row, expansion: Expansion) {
        if let Some(slot) = tree::expansion_mut(&mut self.connections, row) {
            *slot = expansion;
        }
    }
}
