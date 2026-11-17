//! Purpose: A request the explorer is asked for about something that is not in
//! its tree, or not in the view it is asked in, queues nothing and changes
//! nothing.

use super::support::{tables_shown, NO_SUCH_CONNECTION, NO_SUCH_TABLE};
use super::*;
use crate::models::database::TableRef;

#[test]
fn schemas_of_a_connection_the_tree_lacks_are_not_asked_for() {
    let mut queue = Requests::default();
    let mut explorer = loaded(&mut queue);
    explorer.load_schemas(&mut queue, 9);
    explorer.request_schemas(&mut queue, 9);
    assert!(queue.is_idle());
    assert!(!explorer.is_loading());
}

#[test]
fn schemas_already_held_are_not_asked_for_again_by_right() {
    let mut queue = Requests::default();
    let mut explorer = loaded(&mut queue);
    explorer.collapse();
    explorer.expand(&mut queue);
    assert!(queue.is_idle());
    assert_eq!(explorer.rows().len(), 2);
}

#[test]
fn columns_of_a_table_the_tree_lacks_are_not_asked_for() {
    let mut queue = Requests::default();
    let mut explorer = loaded(&mut queue);
    assert!(!explorer.request_columns(&mut queue, NO_SUCH_TABLE));
    assert!(!explorer.request_columns(&mut queue, NO_SUCH_CONNECTION));
    assert!(queue.is_idle());
}

#[test]
fn rows_of_a_table_the_tree_lacks_are_not_opened() {
    let mut queue = Requests::default();
    let mut explorer = loaded(&mut queue);
    explorer.open_rows(&mut queue, NO_SUCH_TABLE);
    explorer.open_rows(&mut queue, NO_SUCH_CONNECTION);
    assert!(queue.is_idle());
    assert!(matches!(explorer.view(), View::Tree));
}

#[test]
fn the_page_keys_do_nothing_outside_the_rows_view() {
    let mut queue = Requests::default();
    let mut explorer = tables_shown(&mut queue);
    explorer.page_next(&mut queue);
    explorer.page_previous(&mut queue);
    explorer.turn_page(&mut queue, 200);
    assert!(queue.is_idle());

    move_to(&mut explorer, items_row());
    explorer.columns(&mut queue);
    answer(&mut explorer, &mut queue, columns());
    explorer.page_next(&mut queue);
    explorer.page_previous(&mut queue);
    assert!(queue.is_idle());
    assert!(matches!(explorer.view(), View::Columns(_)));
}

#[test]
fn a_page_of_a_table_the_tree_lost_is_not_asked_for() {
    let mut queue = Requests::default();
    let mut explorer = tables_shown(&mut queue);
    explorer.view = View::Rows(RowsView {
        path: NO_SUCH_CONNECTION,
        table: TableRef {
            schema: Name::new("main").expect("a name"),
            name: Name::new("gone").expect("a name"),
        },
        offset: 200,
        page: Load::Ready(Page {
            columns: Vec::new(),
            rows: Vec::new(),
            end: PageEnd::Last,
        }),
        scroll: 0,
    });

    explorer.page_previous(&mut queue);
    assert!(queue.is_idle());
    assert!(matches!(explorer.view(), View::Rows(rows) if rows.offset == 200));
}
