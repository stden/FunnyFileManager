//! Purpose: The explorer's views: a table's rows and their pages, its columns,
//! and stepping back from a view to the tree.

use super::*;

#[test]
fn enter_on_a_table_opens_its_rows_and_asks_for_the_first_page() {
    let mut queue = Requests::default();
    let mut explorer = loaded(&mut queue);
    explorer.move_down();
    explorer.expand(&mut queue);
    move_to(&mut explorer, items_row());
    explorer.open(&mut queue);
    assert!(matches!(explorer.view(), View::Rows(_)));
    let request = queue.take().expect("the first page");
    assert!(matches!(request.task, Task::Rows { offset: 0, .. }));
}

#[test]
fn page_down_moves_only_while_the_last_page_said_more() {
    let mut queue = Requests::default();
    let mut explorer = loaded(&mut queue);
    explorer.move_down();
    explorer.expand(&mut queue);
    move_to(&mut explorer, items_row());
    explorer.open(&mut queue);
    answer(&mut explorer, &mut queue, page(PageEnd::Last));
    explorer.page_next(&mut queue);
    assert!(queue.take().is_none());

    let mut queue = Requests::default();
    let mut explorer = loaded(&mut queue);
    explorer.move_down();
    explorer.expand(&mut queue);
    move_to(&mut explorer, items_row());
    explorer.open(&mut queue);
    answer(&mut explorer, &mut queue, page(PageEnd::More));
    explorer.page_next(&mut queue);
    let request = queue.take().expect("the next page");
    assert!(matches!(request.task, Task::Rows { offset: 200, .. }));
}

#[test]
fn tab_and_c_show_the_columns_of_the_table_under_the_cursor() {
    let mut queue = Requests::default();
    let mut explorer = loaded(&mut queue);
    explorer.move_down();
    explorer.expand(&mut queue);
    move_to(&mut explorer, items_row());
    explorer.columns(&mut queue);
    assert!(matches!(explorer.view(), View::Columns(_)));
    answer(&mut explorer, &mut queue, columns());
    assert_eq!(explorer.rows().len(), 4);
    assert!(matches!(
        explorer
            .table(TablePath {
                connection: 0,
                schema: 0,
                table: 0
            })
            .map(|table| &table.columns),
        Some(Load::Ready(_))
    ));
}

#[test]
fn esc_steps_back_to_the_tree_before_the_dialog_closes() {
    let mut queue = Requests::default();
    let mut explorer = loaded(&mut queue);
    explorer.move_down();
    explorer.expand(&mut queue);
    move_to(&mut explorer, items_row());
    explorer.open(&mut queue);
    assert_eq!(explorer.back(), Back::ToTree);
    assert!(matches!(explorer.view(), View::Tree));
    assert_eq!(explorer.back(), Back::Close);
}
