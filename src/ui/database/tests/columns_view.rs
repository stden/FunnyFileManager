//! Purpose: The columns view: one table's columns with their type, key and
//! nullability, while they load, and when the request failed.

use super::{answer, item_columns, on_items, screen, shop, shop_tables, texts};
use crate::models::database::{DatabaseExplorer, Requests, TablePath, View};
use crate::ui::database::columns;

const ITEMS: TablePath = TablePath {
    connection: 0,
    schema: 0,
    table: 0,
};

/// A line of the columns view: the four cells at their widths.
fn cells(name: &str, data_type: &str, key: &str, nullable: &str) -> String {
    format!("{name:<24} {data_type:<20} {key:<12} {nullable}")
}

fn columns_lines(explorer: &DatabaseExplorer) -> Vec<String> {
    let View::Columns(view) = explorer.view() else {
        panic!("the columns view is open")
    };
    texts(&columns::lines(explorer, view.path, view.scroll))
}

#[test]
fn the_columns_view_lists_name_type_key_and_nullability() {
    let mut queue = Requests::default();
    let mut explorer = shop_tables(&mut queue);
    on_items(&mut explorer);
    explorer.columns(&mut queue);
    let header = cells("Name", "Type", "Primary key", "Nullable");
    assert_eq!(
        columns_lines(&explorer),
        ["Columns of items", header.as_str(), "Loading…"]
    );
    assert!(screen(&explorer).contains("Columns of items"));

    answer(&mut explorer, &mut queue, item_columns());
    assert_eq!(
        columns_lines(&explorer),
        [
            "Columns of items",
            header.as_str(),
            cells("id", "INTEGER", "yes", "no").as_str(),
            cells("note", "TEXT", "", "yes").as_str(),
        ]
    );
    explorer.move_down();
    assert_eq!(
        columns_lines(&explorer)[2],
        cells("note", "TEXT", "", "yes")
    );
}

#[test]
fn a_failed_columns_request_shows_the_reason() {
    let mut queue = Requests::default();
    let mut explorer = shop_tables(&mut queue);
    on_items(&mut explorer);
    explorer.columns(&mut queue);
    let request = queue.take().expect("the columns request");
    assert!(queue.finish(request.id));
    explorer.receive(request.id, Err("permission denied".to_owned()));
    assert_eq!(columns_lines(&explorer)[2], "permission denied");
}

#[test]
fn the_columns_of_a_table_the_tree_lacks_show_as_loading() {
    let explorer = DatabaseExplorer::new(vec![shop()], None);
    let lines = texts(&columns::lines(&explorer, ITEMS, 0));
    assert_eq!(lines[0], "");
    assert_eq!(lines[2], "Loading…");
}
