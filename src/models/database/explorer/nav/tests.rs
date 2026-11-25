//! Purpose: Unit tests for `collapse` on a root connection and busy-queue guards on `open`/`columns`.

use super::{DatabaseExplorer, Load, Row, View};
use crate::models::database::{
    ConnectionSpec, Driver, Name, Reply, Requests, Schema, Table, TableKind, Task,
};

fn table_explorer() -> DatabaseExplorer {
    let spec = ConnectionSpec {
        name: Name::new("local").expect("name"),
        driver: Driver::Sqlite {
            path: "/data/x.db".into(),
        },
    };
    let mut queue = Requests::default();
    let mut explorer = DatabaseExplorer::new(vec![spec], None);
    explorer.expand(&mut queue);
    let req = queue.take().expect("schemas");
    assert!(queue.finish(req.id));
    let schema = Schema {
        name: Name::new("main").expect("name"),
        tables: vec![Table {
            name: Name::new("items").expect("name"),
            kind: TableKind::Table,
        }],
    };
    explorer.receive(req.id, Ok(Reply::Schemas(vec![schema])));
    explorer
}

#[test]
fn collapse_on_collapsed_connection_is_a_no_op() {
    let mut explorer = table_explorer();
    explorer.collapse();
    assert!(!explorer.is_open(Row::Connection(0)));
    explorer.collapse();
    assert_eq!(explorer.cursor(), 0);
}

#[test]
fn open_and_columns_keep_tree_view_when_queue_is_busy() {
    let mut explorer = table_explorer();
    let mut queue = Requests::default();
    explorer.move_down();
    explorer.expand(&mut queue);
    explorer.move_down();
    assert!(matches!(explorer.rows()[explorer.cursor()], Row::Table(_)));

    let busy_spec = ConnectionSpec {
        name: Name::new("other").expect("name"),
        driver: Driver::Sqlite {
            path: "/data/other.db".into(),
        },
    };
    queue.queue(Task::Schemas(busy_spec)).expect("occupy queue");
    explorer.open(&mut queue);
    assert!(matches!(explorer.view(), View::Tree));
    explorer.columns(&mut queue);
    assert!(matches!(explorer.view(), View::Tree));
    assert!(matches!(explorer.connections()[0].schemas, Load::Ready(_)));
}
