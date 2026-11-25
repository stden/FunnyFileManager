//! Purpose: The cursor keys in every view: Up and Down walk the tree or scroll
//! a view within its lines, and the tree's keys do nothing where they do not
//! apply: inside a view, on a column, or over an empty tree.

use super::support::{columns_of, items_with_columns, page_of, scroll, tables_shown};
use super::*;

#[test]
fn up_and_down_walk_the_tree_and_stop_at_both_ends() {
    let mut queue = Requests::default();
    let mut explorer = tables_shown(&mut queue);
    assert_eq!(explorer.cursor(), 1);
    explorer.move_up();
    explorer.move_up();
    assert_eq!(explorer.cursor(), 0, "the first row is the top");
    for _ in 0..10 {
        explorer.move_down();
    }
    assert_eq!(explorer.cursor(), 3, "the last row is the bottom");
}

#[test]
fn up_and_down_scroll_the_columns_within_their_lines() {
    let mut queue = Requests::default();
    let mut explorer = tables_shown(&mut queue);
    move_to(&mut explorer, items_row());
    explorer.columns(&mut queue);
    explorer.move_down();
    assert_eq!(scroll(&explorer), 0, "nothing to scroll while they load");

    answer(&mut explorer, &mut queue, columns_of(&["a", "b", "c"]));
    for _ in 0..5 {
        explorer.move_down();
    }
    assert_eq!(scroll(&explorer), 2, "the last of three columns is the end");
    explorer.move_up();
    assert_eq!(scroll(&explorer), 1);
    for _ in 0..5 {
        explorer.move_up();
    }
    assert_eq!(scroll(&explorer), 0);
    assert_eq!(explorer.cursor(), 2, "the tree's cursor stays where it was");
}

#[test]
fn up_and_down_scroll_the_rows_within_the_page() {
    let mut queue = Requests::default();
    let mut explorer = tables_shown(&mut queue);
    move_to(&mut explorer, items_row());
    explorer.open(&mut queue);
    explorer.move_down();
    assert_eq!(
        scroll(&explorer),
        0,
        "nothing to scroll while the page loads"
    );

    answer(&mut explorer, &mut queue, page_of(3, PageEnd::Last));
    for _ in 0..5 {
        explorer.move_down();
    }
    assert_eq!(scroll(&explorer), 2, "the last of three rows is the end");
    explorer.move_up();
    assert_eq!(scroll(&explorer), 1);
    for _ in 0..5 {
        explorer.move_up();
    }
    assert_eq!(scroll(&explorer), 0);
}

#[test]
fn an_empty_tree_has_nothing_to_open_close_show_or_reload() {
    let mut queue = Requests::default();
    let mut explorer = DatabaseExplorer::new(Vec::new(), Some("No connections".to_owned()));
    explorer.expand(&mut queue);
    explorer.collapse();
    explorer.open(&mut queue);
    explorer.columns(&mut queue);
    explorer.reload(&mut queue);
    explorer.move_down();
    explorer.move_up();
    assert!(queue.is_idle());
    assert!(!explorer.is_loading());
    assert_eq!(explorer.cursor(), 0);
    assert_eq!(explorer.notice(), Some("No connections"));
}

#[test]
fn a_column_row_neither_opens_nor_asks_for_more() {
    let mut queue = Requests::default();
    let mut explorer = items_with_columns(&mut queue, &["id", "name"]);
    assert_eq!(explorer.rows().len(), 6);
    explorer.move_down();
    let column = explorer.rows()[explorer.cursor()];
    assert!(matches!(column, Row::Column(..)));

    explorer.expand(&mut queue);
    explorer.open(&mut queue);
    assert!(queue.is_idle());
    assert_eq!(explorer.rows().len(), 6);
    assert!(!explorer.is_open(column));

    explorer.collapse();
    assert_eq!(explorer.rows()[explorer.cursor()], items_row());
}

#[test]
fn enter_opens_and_closes_a_connection_and_a_schema() {
    let mut queue = Requests::default();
    let mut explorer = loaded(&mut queue);
    explorer.open(&mut queue);
    assert_eq!(explorer.rows().len(), 1, "an open connection closes");
    explorer.open(&mut queue);
    assert_eq!(explorer.rows().len(), 2, "a closed one opens again");
    assert!(queue.is_idle(), "its schemas are kept, not asked for again");

    explorer.move_down();
    explorer.open(&mut queue);
    assert_eq!(explorer.rows().len(), 4, "a closed schema shows its tables");
    explorer.open(&mut queue);
    assert_eq!(explorer.rows().len(), 2, "an open schema hides them");
}

#[test]
fn tab_needs_a_table_under_the_cursor() {
    let mut queue = Requests::default();
    let mut explorer = tables_shown(&mut queue);
    explorer.columns(&mut queue);
    assert!(
        matches!(explorer.view(), View::Tree),
        "the cursor is on a schema"
    );
    assert!(queue.is_idle());
}

#[test]
fn the_tree_keys_do_nothing_inside_a_rows_view() {
    let mut queue = Requests::default();
    let mut explorer = tables_shown(&mut queue);
    move_to(&mut explorer, items_row());
    explorer.open(&mut queue);
    answer(&mut explorer, &mut queue, page_of(1, PageEnd::Last));
    let cursor = explorer.cursor();

    explorer.expand(&mut queue);
    explorer.collapse();
    explorer.open(&mut queue);
    explorer.columns(&mut queue);
    explorer.reload(&mut queue);

    assert!(queue.is_idle(), "no key asked for anything");
    assert!(matches!(explorer.view(), View::Rows(_)));
    assert_eq!(explorer.cursor(), cursor);
    assert!(!explorer.is_open(items_row()));
}

#[test]
fn f5_on_a_table_reads_its_connection_again_and_the_cursor_stays_inside() {
    let mut queue = Requests::default();
    let mut explorer = tables_shown(&mut queue);
    move_to(&mut explorer, items_row());
    explorer.reload(&mut queue);
    let request = queue.take().expect("a reload");
    assert!(matches!(request.task, Task::Schemas(ref found) if found.name.as_str() == "local"));
    assert!(queue.finish(request.id));
    explorer.receive(request.id, Ok(main_schema()));
    assert_eq!(
        explorer.rows().len(),
        2,
        "the schemas are read fresh, closed"
    );
    assert_eq!(
        explorer.cursor(),
        1,
        "the cursor is clamped to the last row"
    );
}
