//! Purpose: The explorer's cursor and tree keys: Up and Down move through the
//! tree or scroll a view, Right and Left expand and collapse, Enter opens a
//! table's rows, and Tab or c shows its columns.

use super::tree::{self, Expansion, Load, Row, TablePath};
use super::view::{ColumnsView, RowsView, Want};
use super::{DatabaseExplorer, View};
use crate::models::database::Requests;

impl DatabaseExplorer {
    pub(crate) fn move_up(&mut self) {
        match &mut self.view {
            View::Tree => self.cursor = self.cursor.saturating_sub(1),
            View::Columns(columns) => columns.scroll = columns.scroll.saturating_sub(1),
            View::Rows(rows) => rows.scroll = rows.scroll.saturating_sub(1),
        }
    }

    pub(crate) fn move_down(&mut self) {
        let last_row = self.rows().len().saturating_sub(1);
        let last_line = self.lines().saturating_sub(1);
        match &mut self.view {
            View::Tree => self.cursor = self.cursor.saturating_add(1).min(last_row),
            View::Columns(columns) => {
                columns.scroll = columns.scroll.saturating_add(1).min(last_line);
            }
            View::Rows(rows) => rows.scroll = rows.scroll.saturating_add(1).min(last_line),
        }
    }

    /// The lines the open view holds, for scrolling.
    fn lines(&self) -> usize {
        match &self.view {
            View::Tree => 0,
            View::Columns(columns) => tree::table_at(&self.connections, columns.path)
                .and_then(|table| table.columns.ready())
                .map_or(0, Vec::len),
            View::Rows(rows) => rows.page.ready().map_or(0, |page| page.rows.len()),
        }
    }

    /// Right: open the row under the cursor and ask for what it still lacks.
    pub(crate) fn expand(&mut self, queue: &mut Requests) {
        if !matches!(self.view, View::Tree) {
            return;
        }
        match self.current() {
            Some(Row::Connection(connection)) => {
                self.set_expansion(Row::Connection(connection), Expansion::Expanded);
                self.load_schemas(queue, connection);
            }
            Some(row @ Row::Schema(..)) => self.set_expansion(row, Expansion::Expanded),
            Some(Row::Table(path)) => {
                self.set_expansion(Row::Table(path), Expansion::Expanded);
                self.request_columns(queue, path);
            }
            Some(Row::Column(..)) | None => {}
        }
        self.clamp_cursor();
    }

    /// Left: close the open row under the cursor, or else move to its parent.
    pub(crate) fn collapse(&mut self) {
        if !matches!(self.view, View::Tree) {
            return;
        }
        let Some(row) = self.current() else {
            return;
        };
        if tree::is_open(&self.connections, row) {
            self.set_expansion(row, Expansion::Collapsed);
        } else if let Some(parent) = tree::parent(row) {
            self.cursor = self.position_of(parent);
        }
        self.clamp_cursor();
    }

    /// Enter: a table opens its rows; any other row toggles like Right and Left.
    pub(crate) fn open(&mut self, queue: &mut Requests) {
        if !matches!(self.view, View::Tree) {
            return;
        }
        match self.current() {
            Some(Row::Table(path)) => self.open_rows(queue, path),
            Some(row) if tree::is_open(&self.connections, row) => self.collapse(),
            Some(_) => self.expand(queue),
            None => {}
        }
    }

    pub(super) fn open_rows(&mut self, queue: &mut Requests, path: TablePath) {
        let Some((table, task)) = self.table_ref(path).and_then(|table| {
            let task = self.rows_task(path, &table, 0)?;
            Some((table, task))
        }) else {
            return;
        };
        if self.request(queue, task, Want::Rows) {
            self.view = View::Rows(RowsView {
                path,
                table,
                offset: 0,
                page: Load::Unloaded,
                scroll: 0,
            });
        }
    }

    /// Tab and c: show the columns of the table under the cursor.
    pub(crate) fn columns(&mut self, queue: &mut Requests) {
        if !matches!(self.view, View::Tree) {
            return;
        }
        let Some(Row::Table(path)) = self.current() else {
            return;
        };
        if self.request_columns(queue, path) {
            self.view = View::Columns(ColumnsView { path, scroll: 0 });
        }
    }

    /// F5: ask again for the metadata of the connection under the cursor.
    pub(crate) fn reload(&mut self, queue: &mut Requests) {
        if !matches!(self.view, View::Tree) {
            return;
        }
        let Some(row) = self.current() else {
            return;
        };
        self.request_schemas(queue, tree::connection_of(row));
    }
}
