//! Purpose: Unit tests for stale targets in `apply` and `fail` when the tree or view moved on.

use super::{DatabaseExplorer, Load, Reply, Requests, TablePath, Want};
use crate::models::database::{
    ConnectionSpec, Driver, Name, Page, PageEnd, Schema, Table, TableKind,
};

fn explorer() -> DatabaseExplorer {
    let spec = ConnectionSpec {
        name: Name::new("local").expect("name"),
        driver: Driver::Sqlite {
            path: "/data/x.db".into(),
        },
    };
    DatabaseExplorer::new(vec![spec], None)
}

fn table_ready_explorer(queue: &mut Requests) -> DatabaseExplorer {
    let mut explorer = explorer();
    let schema = Schema {
        name: Name::new("main").expect("name"),
        tables: vec![Table {
            name: Name::new("items").expect("name"),
            kind: TableKind::Table,
        }],
    };
    explorer.expand(queue);
    let req = queue.take().expect("schemas request");
    assert!(queue.finish(req.id));
    assert_eq!(
        explorer.receive(req.id, Ok(Reply::Schemas(vec![schema]))),
        None
    );
    explorer.move_down();
    explorer.expand(queue);
    explorer.move_down();
    explorer
}

#[test]
fn apply_and_fail_ignore_missing_connection_or_table_targets() {
    let mut explorer = explorer();
    let missing = TablePath {
        connection: 0,
        schema: 9,
        table: 0,
    };
    assert_eq!(
        explorer.apply(Want::Schemas(9), Reply::Schemas(Vec::new())),
        None
    );
    assert_eq!(
        explorer.apply(Want::Columns(missing), Reply::Columns(Vec::new())),
        None
    );
    explorer.fail(Want::Schemas(9), "gone");
    explorer.fail(Want::Columns(missing), "gone");
    assert!(matches!(explorer.connections()[0].schemas, Load::Unloaded));
}

#[test]
fn rows_reply_and_failure_are_ignored_after_backing_out_to_tree() {
    let mut queue = Requests::default();
    let mut explorer = table_ready_explorer(&mut queue);
    explorer.open(&mut queue);
    explorer.back();
    let req = queue.take().expect("rows request");
    assert!(queue.finish(req.id));
    let page = Page {
        columns: vec!["id".to_owned()],
        rows: vec![vec!["1".to_owned()]],
        end: PageEnd::Last,
    };
    assert_eq!(explorer.receive(req.id, Ok(Reply::Rows(page))), None);

    explorer.open(&mut queue);
    explorer.back();
    let req2 = queue.take().expect("second rows request");
    assert!(queue.finish(req2.id));
    assert_eq!(
        explorer.receive(req2.id, Err("boom".to_owned())).as_deref(),
        Some("boom")
    );
}
