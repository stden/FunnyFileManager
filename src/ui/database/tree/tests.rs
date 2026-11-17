//! Purpose: The tree's text for a row the explorer does not hold: it draws
//! nothing for it rather than failing.

use super::{column_detail, connection_detail, table_detail};
use crate::models::database::{Requests, TablePath};
use crate::ui::database::tests::shop_tables;

const NOWHERE: TablePath = TablePath {
    connection: 0,
    schema: 3,
    table: 0,
};

#[test]
fn a_row_the_explorer_does_not_hold_has_no_detail() {
    let mut queue = Requests::default();
    let explorer = shop_tables(&mut queue);
    assert_eq!(connection_detail(&explorer, 5), "");
    assert_eq!(table_detail(&explorer, NOWHERE), "");
    assert_eq!(column_detail(&explorer, NOWHERE, 0), "");
}

#[test]
fn a_column_not_loaded_yet_has_no_detail() {
    let mut queue = Requests::default();
    let explorer = shop_tables(&mut queue);
    let items = TablePath {
        schema: 0,
        ..NOWHERE
    };
    assert_eq!(column_detail(&explorer, items, 0), "");
}
