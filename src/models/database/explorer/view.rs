//! Purpose: What the explorer shows over its tree: the tree itself, one table's
//! columns, or one page of its rows, and the request each of them waits for.

use super::tree::{Load, TablePath};
use crate::models::database::{Page, RequestId, TableRef};

#[derive(Debug, Clone)]
pub(crate) enum View {
    Tree,
    Columns(ColumnsView),
    Rows(RowsView),
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct ColumnsView {
    pub(crate) path: TablePath,
    pub(crate) scroll: usize,
}

#[derive(Debug, Clone)]
pub(crate) struct RowsView {
    pub(crate) path: TablePath,
    pub(crate) table: TableRef,
    pub(crate) offset: usize,
    pub(crate) page: Load<Page>,
    pub(crate) scroll: usize,
}

/// Where Esc leads: back to the tree, or out of the dialog from the tree.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Back {
    ToTree,
    Close,
}

/// The node a request answers for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Want {
    Schemas(usize),
    Columns(TablePath),
    Rows,
}

#[derive(Debug, Clone, Copy)]
pub(super) struct Pending {
    pub(super) id: RequestId,
    pub(super) want: Want,
}
