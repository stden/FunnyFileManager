//! Purpose: Applying or recording failure for an awaited explorer reply.

use super::super::tree::{self, Load, SchemaNode};
use super::super::view::Want;
use super::super::{DatabaseExplorer, View};
use crate::models::database::Reply;

const MISMATCH: &str = "The database answered with something else";

impl DatabaseExplorer {
    pub(super) fn apply(&mut self, want: Want, reply: Reply) -> Option<String> {
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

    pub(super) fn fail(&mut self, want: Want, message: &str) {
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
