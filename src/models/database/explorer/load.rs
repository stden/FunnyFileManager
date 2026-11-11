//! Purpose: Asking for what the tree and the views lack, and taking in the
//! answer. A request is queued once; its answer is applied to the node it was
//! asked for, and an answer nobody waits for is dropped.

use super::tree::{self, Load, SchemaNode, TablePath};
use super::view::{Pending, Want};
use super::{DatabaseExplorer, View};
use crate::models::database::{PageEnd, Reply, RequestId, Requests, TableRef, Task, PAGE_ROWS};

const BUSY: &str = "A database request is still running; try again";
const MISMATCH: &str = "The database answered with something else";

impl DatabaseExplorer {
    /// Queue `task` for `want`. `false` while a request is pending, or when the
    /// queue is busy with a request from a dialog that was closed.
    pub(super) fn request(&mut self, queue: &mut Requests, task: Task, want: Want) -> bool {
        if self.pending.is_some() {
            return false;
        }
        let Some(id) = queue.queue(task) else {
            self.notice = Some(BUSY.to_owned());
            return false;
        };
        self.pending = Some(Pending { id, want });
        true
    }

    pub(super) fn load_schemas(&mut self, queue: &mut Requests, connection: usize) {
        let loaded = self
            .connections
            .get(connection)
            .is_some_and(|node| node.schemas.ready().is_some());
        if !loaded {
            self.request_schemas(queue, connection);
        }
    }

    /// Ask for a connection's schemas, whether or not it holds them already.
    pub(super) fn request_schemas(&mut self, queue: &mut Requests, connection: usize) {
        let Some(node) = self.connections.get(connection) else {
            return;
        };
        let task = Task::Schemas(node.spec.clone());
        self.request(queue, task, Want::Schemas(connection));
    }

    /// `true` when the columns are loaded already, or their request is queued
    /// now; `false` when nothing was queued.
    pub(super) fn request_columns(&mut self, queue: &mut Requests, path: TablePath) -> bool {
        if tree::table_at(&self.connections, path)
            .is_some_and(|table| table.columns.ready().is_some())
        {
            return true;
        }
        let Some(spec) = self
            .connections
            .get(path.connection)
            .map(|node| node.spec.clone())
        else {
            return false;
        };
        let Some(table) = self.table_ref(path) else {
            return false;
        };
        self.request(queue, Task::Columns { spec, table }, Want::Columns(path))
    }

    pub(super) fn table_ref(&self, path: TablePath) -> Option<TableRef> {
        let schema = tree::schema_at(&self.connections, path.connection, path.schema)?;
        let table = tree::table_at(&self.connections, path)?;
        Some(TableRef {
            schema: schema.name.clone(),
            name: table.name.clone(),
        })
    }

    pub(super) fn rows_task(
        &self,
        path: TablePath,
        table: &TableRef,
        offset: usize,
    ) -> Option<Task> {
        let spec = self.connections.get(path.connection)?.spec.clone();
        Some(Task::Rows {
            spec,
            table: table.clone(),
            offset,
        })
    }

    /// `PgDn`: the next page, when the last one said rows follow.
    pub(crate) fn page_next(&mut self, queue: &mut Requests) {
        let View::Rows(rows) = &self.view else {
            return;
        };
        let more = matches!(rows.page.ready(), Some(page) if page.end == PageEnd::More);
        if !more {
            return;
        }
        let offset = rows.offset.saturating_add(PAGE_ROWS);
        self.turn_page(queue, offset);
    }

    /// `PgUp`: the page before, when there is one.
    pub(crate) fn page_previous(&mut self, queue: &mut Requests) {
        let View::Rows(rows) = &self.view else {
            return;
        };
        if rows.offset == 0 {
            return;
        }
        let offset = rows.offset.saturating_sub(PAGE_ROWS);
        self.turn_page(queue, offset);
    }

    pub(super) fn turn_page(&mut self, queue: &mut Requests, offset: usize) {
        let View::Rows(rows) = &self.view else {
            return;
        };
        let Some(task) = self.rows_task(rows.path, &rows.table, offset) else {
            return;
        };
        if !self.request(queue, task, Want::Rows) {
            return;
        }
        if let View::Rows(rows) = &mut self.view {
            rows.offset = offset;
            rows.page = Load::Unloaded;
            rows.scroll = 0;
        }
    }

    /// Take the answer to request `id`. Returns the message to show when the
    /// request failed or answered with the wrong kind; `None` when it was applied,
    /// or dropped because nothing waits for `id`.
    pub(crate) fn receive(
        &mut self,
        id: RequestId,
        answer: Result<Reply, String>,
    ) -> Option<String> {
        let pending = self.pending.take_if(|pending| pending.id == id)?;
        self.notice = None;
        let message = match answer {
            Ok(reply) => self.apply(pending.want, reply),
            Err(message) => {
                self.fail(pending.want, &message);
                Some(message)
            }
        };
        self.clamp_cursor();
        message
    }

    fn apply(&mut self, want: Want, reply: Reply) -> Option<String> {
        match (want, reply) {
            (Want::Schemas(connection), Reply::Schemas(schemas)) => {
                if let Some(node) = self.connections.get_mut(connection) {
                    let nodes = schemas.into_iter().map(SchemaNode::from_schema).collect();
                    node.schemas = Load::Ready(nodes);
                }
                None
            }
            (Want::Columns(path), Reply::Columns(columns)) => {
                if let Some(table) = tree::table_mut(&mut self.connections, path) {
                    table.columns = Load::Ready(columns);
                }
                None
            }
            (Want::Rows, Reply::Rows(page)) => {
                if let View::Rows(rows) = &mut self.view {
                    rows.page = Load::Ready(page);
                }
                None
            }
            _ => Some(MISMATCH.to_owned()),
        }
    }

    fn fail(&mut self, want: Want, message: &str) {
        match want {
            Want::Schemas(connection) => {
                if let Some(node) = self.connections.get_mut(connection) {
                    node.schemas = Load::Failed(message.to_owned());
                }
            }
            Want::Columns(path) => {
                if let Some(table) = tree::table_mut(&mut self.connections, path) {
                    table.columns = Load::Failed(message.to_owned());
                }
            }
            Want::Rows => {
                if let View::Rows(rows) = &mut self.view {
                    rows.page = Load::Failed(message.to_owned());
                }
            }
        }
    }
}
