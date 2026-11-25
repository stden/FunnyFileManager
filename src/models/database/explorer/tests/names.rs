//! Purpose: The names the tree shows for its rows, and which row a pending
//! request is loading.

use super::support::{items_with_columns, tables_shown, ITEMS, NO_SUCH_TABLE};
use super::*;

#[test]
fn every_row_shows_the_name_the_database_reported() {
    let mut queue = Requests::default();
    let explorer = items_with_columns(&mut queue, &["id", "title"]);
    let names: Vec<_> = explorer
        .rows()
        .into_iter()
        .map(|row| explorer.name(row))
        .collect();
    assert_eq!(
        names,
        [
            Some("local"),
            Some("main"),
            Some("items"),
            Some("id"),
            Some("title"),
            Some("item_labels")
        ]
    );
}

#[test]
fn a_row_the_tree_does_not_hold_has_no_name() {
    let mut queue = Requests::default();
    let explorer = items_with_columns(&mut queue, &["id"]);
    assert_eq!(explorer.name(Row::Connection(9)), None);
    assert_eq!(explorer.name(Row::Schema(0, 5)), None);
    assert_eq!(explorer.name(Row::Table(NO_SUCH_TABLE)), None);
    assert_eq!(explorer.name(Row::Column(ITEMS, 7)), None);
    let labels = TablePath { table: 1, ..ITEMS };
    assert_eq!(explorer.name(Row::Column(labels, 0)), None, "not loaded");
}

#[test]
fn a_pending_request_loads_only_the_row_it_fills() {
    let mut queue = Requests::default();
    let mut explorer = DatabaseExplorer::new(vec![spec("local"), spec("other")], None);
    assert!(!explorer.is_loading_row(Row::Connection(0)));
    explorer.expand(&mut queue);
    assert!(explorer.is_loading_row(Row::Connection(0)));
    assert!(!explorer.is_loading_row(Row::Connection(1)));
    assert!(!explorer.is_loading_row(Row::Schema(0, 0)));
    assert!(!explorer.is_loading_row(Row::Table(ITEMS)));

    let mut queue = Requests::default();
    let mut explorer = tables_shown(&mut queue);
    move_to(&mut explorer, items_row());
    explorer.expand(&mut queue);
    assert!(explorer.is_loading_row(Row::Table(ITEMS)));
    assert!(!explorer.is_loading_row(Row::Table(TablePath { table: 1, ..ITEMS })));
    assert!(!explorer.is_loading_row(Row::Connection(0)));
    assert!(!explorer.is_loading_row(Row::Column(ITEMS, 0)));
}
