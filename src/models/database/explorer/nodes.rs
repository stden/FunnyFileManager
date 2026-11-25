//! Purpose: The nodes of the explorer's metadata tree: each connection, schema
//! and table, what each one holds, and the `Row`s that name them.

use crate::models::database::{Column, ConnectionSpec, Name, Schema, Table, TableKind};

/// Whether a row's children are shown.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) enum Expansion {
    Expanded,
    #[default]
    Collapsed,
}

/// What a node holds once its request has answered.
#[derive(Debug, Clone)]
pub(crate) enum Load<T> {
    Unloaded,
    Ready(T),
    Failed(String),
}

impl<T> Load<T> {
    pub(crate) const fn ready(&self) -> Option<&T> {
        match self {
            Self::Ready(value) => Some(value),
            Self::Unloaded | Self::Failed(_) => None,
        }
    }
}

#[derive(Debug, Clone)]
pub(crate) struct Connection {
    pub(crate) spec: ConnectionSpec,
    pub(crate) expansion: Expansion,
    pub(crate) schemas: Load<Vec<SchemaNode>>,
}

#[derive(Debug, Clone)]
pub(crate) struct SchemaNode {
    pub(crate) name: Name,
    pub(crate) expansion: Expansion,
    pub(crate) tables: Vec<TableNode>,
}

#[derive(Debug, Clone)]
pub(crate) struct TableNode {
    pub(crate) name: Name,
    pub(crate) kind: TableKind,
    pub(crate) expansion: Expansion,
    pub(crate) columns: Load<Vec<Column>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct TablePath {
    pub(crate) connection: usize,
    pub(crate) schema: usize,
    pub(crate) table: usize,
}

/// One line of the tree.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Row {
    Connection(usize),
    Schema(usize, usize),
    Table(TablePath),
    Column(TablePath, usize),
}

impl Connection {
    pub(crate) const fn new(spec: ConnectionSpec) -> Self {
        Self {
            spec,
            expansion: Expansion::Collapsed,
            schemas: Load::Unloaded,
        }
    }
}

impl SchemaNode {
    pub(crate) fn from_schema(schema: Schema) -> Self {
        Self {
            name: schema.name,
            expansion: Expansion::Collapsed,
            tables: schema.tables.into_iter().map(TableNode::new).collect(),
        }
    }
}

impl TableNode {
    fn new(table: Table) -> Self {
        Self {
            name: table.name,
            kind: table.kind,
            expansion: Expansion::Collapsed,
            columns: Load::Unloaded,
        }
    }
}
