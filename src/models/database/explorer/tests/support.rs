//! Purpose: Builders the explorer's key, answer and name tests share: an
//! explorer with its tables showing, and replies of a chosen size.

use super::*;

/// The path of `items`, the first table of `main`.
pub(super) const ITEMS: TablePath = TablePath {
    connection: 0,
    schema: 0,
    table: 0,
};

/// `loaded`, with the schema open: the rows are the connection, `main`, `items`
/// and `item_labels`, and the cursor is on `main`.
pub(super) fn tables_shown(queue: &mut Requests) -> DatabaseExplorer {
    let mut explorer = loaded(queue);
    explorer.move_down();
    explorer.expand(queue);
    explorer
}

/// `tables_shown` with the cursor on `items`, its columns asked for and answered.
pub(super) fn items_with_columns(queue: &mut Requests, names: &[&str]) -> DatabaseExplorer {
    let mut explorer = tables_shown(queue);
    move_to(&mut explorer, items_row());
    explorer.expand(queue);
    answer(&mut explorer, queue, columns_of(names));
    explorer
}

pub(super) fn columns_of(names: &[&str]) -> Reply {
    Reply::Columns(
        names
            .iter()
            .map(|name| Column {
                name: (*name).to_owned(),
                data_type: "TEXT".to_owned(),
                key: KeyRole::Plain,
                nullability: Nullability::Nullable,
            })
            .collect(),
    )
}

/// A page of `count` rows of one cell each.
pub(super) fn page_of(count: usize, end: PageEnd) -> Reply {
    Reply::Rows(Page {
        columns: vec!["id".to_owned()],
        rows: (1..=count).map(|id| vec![id.to_string()]).collect(),
        end,
    })
}

/// A table path no tree holds: the connection exists, the schema does not.
pub(super) const NO_SUCH_TABLE: TablePath = TablePath {
    connection: 0,
    schema: 5,
    table: 0,
};

/// A table path of a connection that does not exist.
pub(super) const NO_SUCH_CONNECTION: TablePath = TablePath {
    connection: 9,
    schema: 0,
    table: 0,
};

pub(super) fn scroll(explorer: &DatabaseExplorer) -> usize {
    match explorer.view() {
        View::Columns(columns) => columns.scroll,
        View::Rows(rows) => rows.scroll,
        View::Tree => panic!("the tree has no scroll"),
    }
}
