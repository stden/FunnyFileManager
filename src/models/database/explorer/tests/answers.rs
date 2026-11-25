//! Purpose: What the explorer does with an answer it cannot use, and with a
//! request it cannot make: a busy queue, an answer of the wrong kind, and a
//! failure on a table's columns or rows.

use super::support::{page_of, tables_shown, ITEMS};
use super::*;

const BUSY: &str = "A database request is still running; try again";

#[test]
fn a_busy_queue_names_the_reason_and_the_next_try_goes_through() {
    let mut queue = Requests::default();
    queue
        .queue(Task::Schemas(spec("from a closed dialog")))
        .expect("the queue is free");
    let mut explorer = DatabaseExplorer::new(vec![spec("local")], None);
    explorer.expand(&mut queue);
    assert!(!explorer.is_loading());
    assert_eq!(explorer.notice(), Some(BUSY));

    let stale = queue.take().expect("the stale request");
    assert!(queue.finish(stale.id));
    explorer.reload(&mut queue);
    assert!(explorer.is_loading());
    assert_eq!(
        explorer.notice(),
        Some(BUSY),
        "kept until an answer replaces it"
    );
    assert_eq!(answer(&mut explorer, &mut queue, main_schema()), None);
    assert_eq!(explorer.notice(), None);
}

#[test]
fn an_answer_of_the_wrong_kind_is_reported_and_leaves_the_tree_alone() {
    let mut queue = Requests::default();
    let mut explorer = DatabaseExplorer::new(vec![spec("local")], None);
    explorer.expand(&mut queue);
    let message = answer(&mut explorer, &mut queue, columns());
    assert_eq!(
        message.as_deref(),
        Some("The database answered with something else")
    );
    assert!(matches!(explorer.connections()[0].schemas, Load::Unloaded));
    assert!(!explorer.is_loading(), "the request is spent either way");
}

#[test]
fn a_failed_columns_request_is_kept_on_the_table() {
    let mut queue = Requests::default();
    let mut explorer = tables_shown(&mut queue);
    move_to(&mut explorer, items_row());
    explorer.expand(&mut queue);
    let request = queue.take().expect("the columns request");
    assert!(queue.finish(request.id));
    let message = explorer.receive(request.id, Err("no such table".to_owned()));
    assert_eq!(message.as_deref(), Some("no such table"));

    let table = explorer.table(ITEMS).expect("the items table");
    assert!(matches!(&table.columns, Load::Failed(text) if text.as_str() == "no such table"));
    assert_eq!(explorer.rows().len(), 4, "a failure adds no column rows");
}

#[test]
fn a_failed_rows_request_is_kept_on_the_page() {
    let mut queue = Requests::default();
    let mut explorer = tables_shown(&mut queue);
    move_to(&mut explorer, items_row());
    explorer.open(&mut queue);
    let request = queue.take().expect("the first page");
    assert!(queue.finish(request.id));
    let message = explorer.receive(request.id, Err("statement timeout".to_owned()));
    assert_eq!(message.as_deref(), Some("statement timeout"));

    let View::Rows(rows) = explorer.view() else {
        panic!("the rows view stays open")
    };
    assert!(matches!(&rows.page, Load::Failed(text) if text.as_str() == "statement timeout"));
}

#[test]
fn a_page_turn_waits_for_the_page_still_loading() {
    let mut queue = Requests::default();
    let mut explorer = tables_shown(&mut queue);
    move_to(&mut explorer, items_row());
    explorer.open(&mut queue);
    answer(&mut explorer, &mut queue, page_of(1, PageEnd::More));
    explorer.page_next(&mut queue);
    explorer.page_previous(&mut queue);

    let View::Rows(rows) = explorer.view() else {
        panic!("the rows view stays open")
    };
    assert_eq!(rows.offset, 200, "the page being read is the one shown");
    let request = queue.take().expect("the next page");
    assert!(matches!(request.task, Task::Rows { offset: 200, .. }));
    assert!(queue.take().is_none(), "PgUp queued nothing behind it");
}
