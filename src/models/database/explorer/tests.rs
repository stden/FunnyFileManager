//! Purpose: Pin the explorer's state: what Right, Left, Enter, Tab, c, `PgDn`,
//! F5 and Esc do to the tree and the views, one request at a time, and which
//! answers are kept.

use super::*;
use crate::models::database::{
    Column, Driver, KeyRole, Name, Nullability, Page, PageEnd, Reply, Requests, Schema, Table,
    TableKind, Task,
};

mod answers;
mod guards;
mod keys;
mod names;
mod support;
mod tree_paths;
mod views;

fn spec(name: &str) -> ConnectionSpec {
    ConnectionSpec {
        name: Name::new(name).expect("a name"),
        driver: Driver::Sqlite {
            path: "/data/x.db".into(),
        },
    }
}

fn table(name: &str, kind: TableKind) -> Table {
    Table {
        name: Name::new(name).expect("a name"),
        kind,
    }
}

/// `main` holding `items` (a table) and `item_labels` (a view).
fn main_schema() -> Reply {
    Reply::Schemas(vec![Schema {
        name: Name::new("main").expect("a name"),
        tables: vec![
            table("items", TableKind::Table),
            table("item_labels", TableKind::View),
        ],
    }])
}

fn columns() -> Reply {
    Reply::Columns(vec![Column {
        name: "id".to_owned(),
        data_type: "INTEGER".to_owned(),
        key: KeyRole::Primary,
        nullability: Nullability::NotNull,
    }])
}

fn page(end: PageEnd) -> Reply {
    Reply::Rows(Page {
        columns: vec!["id".to_owned()],
        rows: vec![vec!["1".to_owned()]],
        end,
    })
}

/// Take the queued request, mark it running, and answer it.
fn answer(explorer: &mut DatabaseExplorer, queue: &mut Requests, reply: Reply) -> Option<String> {
    let request = queue.take().expect("a queued request");
    assert!(queue.finish(request.id));
    explorer.receive(request.id, Ok(reply))
}

/// An explorer over one connection, its schemas loaded and the cursor on it.
fn loaded(queue: &mut Requests) -> DatabaseExplorer {
    let mut explorer = DatabaseExplorer::new(vec![spec("local")], None);
    explorer.expand(queue);
    answer(&mut explorer, queue, main_schema());
    explorer
}

fn move_to(explorer: &mut DatabaseExplorer, row: Row) {
    while explorer.rows().get(explorer.cursor()) != Some(&row) {
        explorer.move_down();
    }
}

fn items_row() -> Row {
    Row::Table(TablePath {
        connection: 0,
        schema: 0,
        table: 0,
    })
}

#[test]
fn right_on_a_connection_opens_it_and_asks_for_its_schemas() {
    let mut queue = Requests::default();
    let mut explorer = DatabaseExplorer::new(vec![spec("local")], None);
    explorer.expand(&mut queue);
    assert_eq!(explorer.connections()[0].expansion, Expansion::Expanded);
    assert!(explorer.is_loading());
    let request = queue.take().expect("a request");
    assert!(matches!(request.task, Task::Schemas(ref found) if found.name.as_str() == "local"));
}

#[test]
fn the_schemas_answer_fills_the_tree_and_the_cursor_walks_it() {
    let mut queue = Requests::default();
    let mut explorer = loaded(&mut queue);
    assert!(!explorer.is_loading());
    assert_eq!(explorer.rows().len(), 2);
    explorer.move_down();
    explorer.expand(&mut queue);
    assert_eq!(explorer.rows().len(), 4);
    assert!(matches!(explorer.rows()[2], Row::Table(_)));
}

#[test]
fn left_collapses_an_open_node_and_then_moves_to_its_parent() {
    let mut queue = Requests::default();
    let mut explorer = loaded(&mut queue);
    explorer.move_down();
    explorer.expand(&mut queue);
    explorer.collapse();
    assert_eq!(explorer.rows().len(), 2);
    assert_eq!(explorer.cursor(), 1);
    explorer.collapse();
    assert_eq!(explorer.cursor(), 0);
}

#[test]
fn right_on_a_table_asks_for_its_columns_once() {
    let mut queue = Requests::default();
    let mut explorer = loaded(&mut queue);
    explorer.move_down();
    explorer.expand(&mut queue);
    move_to(&mut explorer, items_row());
    explorer.expand(&mut queue);
    assert!(explorer.is_loading_row(items_row()));
    let request = queue.take().expect("the columns request");
    assert!(matches!(request.task, Task::Columns { .. }));
    explorer.expand(&mut queue);
    assert!(queue.take().is_none());
}

#[test]
fn a_second_request_is_not_queued_while_one_is_pending() {
    let mut queue = Requests::default();
    let mut explorer = DatabaseExplorer::new(vec![spec("local")], None);
    explorer.expand(&mut queue);
    explorer.reload(&mut queue);
    assert!(queue.take().is_some());
    assert!(queue.take().is_none());
}

#[test]
fn an_answer_nobody_waits_for_is_dropped() {
    let mut queue = Requests::default();
    let mut explorer = DatabaseExplorer::new(vec![spec("local")], None);
    explorer.expand(&mut queue);
    // Ids count up per queue, so the second request of another queue is a
    // stranger to this explorer, whose request is the first one of its own.
    let mut other = Requests::default();
    let first = other
        .queue(Task::Schemas(spec("elsewhere")))
        .expect("a first id");
    assert!(other.take().is_some());
    assert!(other.finish(first));
    let stranger = other
        .queue(Task::Schemas(spec("elsewhere")))
        .expect("a second id");
    assert_eq!(explorer.receive(stranger, Ok(main_schema())), None);
    assert!(explorer.is_loading());
    assert!(matches!(explorer.connections()[0].schemas, Load::Unloaded));
}

#[test]
fn a_failure_is_kept_on_the_node_and_its_message_is_returned() {
    let mut queue = Requests::default();
    let mut explorer = DatabaseExplorer::new(vec![spec("local")], None);
    explorer.expand(&mut queue);
    let request = queue.take().expect("the request");
    assert!(queue.finish(request.id));
    let message = explorer.receive(request.id, Err("unable to open".to_owned()));
    assert_eq!(message.as_deref(), Some("unable to open"));
    assert!(matches!(
        &explorer.connections()[0].schemas,
        Load::Failed(text) if text.as_str() == "unable to open"
    ));
}

#[test]
fn f5_asks_again_for_the_connection_under_the_cursor() {
    let mut queue = Requests::default();
    let mut explorer = loaded(&mut queue);
    explorer.reload(&mut queue);
    let request = queue.take().expect("a reload");
    assert!(matches!(request.task, Task::Schemas(ref found) if found.name.as_str() == "local"));
}
