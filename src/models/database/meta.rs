//! Purpose: What a database reports about itself: schemas, tables and views,
//! columns, and one page of rows with every value as text.

use super::Name;

/// Rows per page of the rows view.
pub(crate) const PAGE_ROWS: usize = 200;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum TableKind {
    Table,
    View,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Table {
    pub(crate) name: Name,
    pub(crate) kind: TableKind,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Schema {
    pub(crate) name: Name,
    pub(crate) tables: Vec<Table>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum KeyRole {
    Primary,
    Plain,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Nullability {
    Nullable,
    NotNull,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Column {
    pub(crate) name: String,
    pub(crate) data_type: String,
    pub(crate) key: KeyRole,
    pub(crate) nullability: Nullability,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct TableRef {
    pub(crate) schema: Name,
    pub(crate) name: Name,
}

/// Whether the table holds rows past this page.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PageEnd {
    More,
    Last,
}

/// One page of rows, every value as text.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Page {
    pub(crate) columns: Vec<String>,
    pub(crate) rows: Vec<Vec<String>>,
    pub(crate) end: PageEnd,
}
