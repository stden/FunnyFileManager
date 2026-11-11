//! Purpose: Pin what the explorer draws: its hint when no connection is
//! configured, a connection's name and driver, and Loading while a request
//! is in flight.

use ratatui::{backend::TestBackend, Terminal};

use super::render;
use crate::models::database::{
    Column, ConnectionSpec, DatabaseExplorer, Driver, KeyRole, Name, Nullability, Reply, Requests,
    Row, Schema, Table, TableKind, Task,
};

mod columns_view;
mod rows_view;
mod tree_lines;

fn screen(explorer: &DatabaseExplorer) -> String {
    let mut terminal = Terminal::new(TestBackend::new(100, 24)).expect("a terminal");
    terminal
        .draw(|frame| render(frame, explorer))
        .expect("a frame");
    terminal
        .backend()
        .buffer()
        .content()
        .iter()
        .map(ratatui::buffer::Cell::symbol)
        .collect()
}

pub(super) fn shop() -> ConnectionSpec {
    ConnectionSpec {
        name: Name::new("shop").expect("a name"),
        driver: Driver::Sqlite {
            path: "/data/shop.db".into(),
        },
    }
}

#[test]
fn an_explorer_without_connections_shows_its_hint() {
    let explorer = DatabaseExplorer::new(
        Vec::new(),
        Some("No connections: create databases.yaml".to_owned()),
    );
    assert!(screen(&explorer).contains("create databases.yaml"));
}

#[test]
fn a_connection_shows_its_name_and_driver() {
    let explorer = DatabaseExplorer::new(vec![shop()], None);
    let text = screen(&explorer);
    assert!(text.contains("shop"));
    assert!(text.contains("sqlite"));
}

#[test]
fn a_request_in_flight_shows_loading() {
    let mut queue = Requests::default();
    let mut explorer = DatabaseExplorer::new(vec![shop()], None);
    explorer.expand(&mut queue);
    assert!(matches!(
        queue.take().map(|request| request.task),
        Some(Task::Schemas(_))
    ));
    assert!(screen(&explorer).contains("Loading"));
}

pub(super) fn texts(lines: &[ratatui::text::Line<'_>]) -> Vec<String> {
    lines.iter().map(ToString::to_string).collect()
}

/// Take the queued request, mark it done, and answer it.
pub(super) fn answer(explorer: &mut DatabaseExplorer, queue: &mut Requests, reply: Reply) {
    let request = queue.take().expect("a queued request");
    assert!(queue.finish(request.id));
    assert_eq!(explorer.receive(request.id, Ok(reply)), None);
}

fn table(name: &str, kind: TableKind) -> Table {
    Table {
        name: Name::new(name).expect("a name"),
        kind,
    }
}

/// `shop` opened: schema `main` open, holding the table `items` and the view
/// `item_labels`. The rows are `shop`, `main`, `items`, `item_labels`; the
/// cursor is on `main`.
pub(super) fn shop_tables(queue: &mut Requests) -> DatabaseExplorer {
    let mut explorer = DatabaseExplorer::new(vec![shop()], None);
    explorer.expand(queue);
    let schemas = Reply::Schemas(vec![Schema {
        name: Name::new("main").expect("a name"),
        tables: vec![
            table("items", TableKind::Table),
            table("item_labels", TableKind::View),
        ],
    }]);
    answer(&mut explorer, queue, schemas);
    explorer.move_down();
    explorer.expand(queue);
    explorer
}

/// Put the cursor on `items`.
pub(super) fn on_items(explorer: &mut DatabaseExplorer) {
    explorer.move_down();
    assert!(matches!(explorer.rows()[explorer.cursor()], Row::Table(_)));
}

/// `id`, the primary key, and `note`, which may be empty.
pub(super) fn item_columns() -> Reply {
    Reply::Columns(vec![
        Column {
            name: "id".to_owned(),
            data_type: "INTEGER".to_owned(),
            key: KeyRole::Primary,
            nullability: Nullability::NotNull,
        },
        Column {
            name: "note".to_owned(),
            data_type: "TEXT".to_owned(),
            key: KeyRole::Plain,
            nullability: Nullability::Nullable,
        },
    ])
}
