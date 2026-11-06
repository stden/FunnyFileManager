//! Purpose: Pin the SQLite driver on a database file the test creates: the
//! metadata it reads, the read-only refusal, paging of 450 rows, and object
//! names that hold quotes, spaces and dots.

use std::path::{Path, PathBuf};

use super::{columns, connect, rows, schemas, tables};
use crate::models::database::{KeyRole, Name, Nullability, PageEnd, TableKind, TableRef};

fn name(text: &str) -> Name {
    Name::new(text).expect("a name")
}

fn table_ref(schema: &str, table: &str) -> TableRef {
    TableRef {
        schema: name(schema),
        name: name(table),
    }
}

/// A database file with a table, a view and a table whose name has a quote,
/// a space and a dot.
fn shop(dir: &tempfile::TempDir) -> PathBuf {
    let path = dir.path().join("shop.db");
    let conn = rusqlite::Connection::open(&path).expect("create the database");
    conn.execute_batch(
        "CREATE TABLE items (id INTEGER PRIMARY KEY, label TEXT NOT NULL, note TEXT);
         CREATE VIEW item_labels AS SELECT label FROM items;
         CREATE TABLE \"odd\"\"name.v 1\" (\"a \"\"b\"\"\" TEXT);",
    )
    .expect("schema");
    path
}

fn add_rows(path: &Path, count: u32) {
    let writer = rusqlite::Connection::open(path).expect("writer");
    writer
        .execute_batch(&format!(
            "CREATE TABLE big (n INTEGER);
             WITH RECURSIVE c(x) AS (SELECT 1 UNION ALL SELECT x + 1 FROM c WHERE x < {count})
             INSERT INTO big (n) SELECT x FROM c;"
        ))
        .expect("rows");
}

#[test]
fn schemas_lists_the_main_database() {
    let dir = tempfile::TempDir::new().expect("temp dir");
    let conn = connect(&shop(&dir)).expect("open");
    let found = schemas(&conn).expect("schemas");
    assert!(found.iter().any(|schema| schema.as_str() == "main"));
}

#[test]
fn tables_and_views_come_with_their_kind_and_internal_tables_are_skipped() {
    let dir = tempfile::TempDir::new().expect("temp dir");
    let conn = connect(&shop(&dir)).expect("open");
    let found = tables(&conn, &name("main")).expect("tables");
    let listed: Vec<(&str, TableKind)> = found
        .iter()
        .map(|table| (table.name.as_str(), table.kind))
        .collect();
    assert_eq!(
        listed,
        vec![
            ("item_labels", TableKind::View),
            ("items", TableKind::Table),
            ("odd\"name.v 1", TableKind::Table),
        ]
    );
}

#[test]
fn columns_carry_their_type_primary_key_and_nullability() {
    let dir = tempfile::TempDir::new().expect("temp dir");
    let conn = connect(&shop(&dir)).expect("open");
    let found = columns(&conn, &table_ref("main", "items")).expect("columns");
    let by_name = |wanted: &str| {
        found
            .iter()
            .find(|column| column.name == wanted)
            .expect("the column")
            .clone()
    };
    assert_eq!(by_name("id").key, KeyRole::Primary);
    assert_eq!(by_name("label").nullability, Nullability::NotNull);
    assert_eq!(by_name("note").nullability, Nullability::Nullable);
    assert_eq!(by_name("label").data_type, "TEXT");
}

#[test]
fn the_connection_refuses_an_insert_and_says_it_is_query_only() {
    let dir = tempfile::TempDir::new().expect("temp dir");
    let path = shop(&dir);
    let conn = connect(&path).expect("open");
    assert!(conn
        .execute("INSERT INTO items (label) VALUES ('x')", [])
        .is_err());
    let query_only: i64 = conn
        .query_row("PRAGMA query_only", [], |row| row.get(0))
        .expect("the pragma");
    assert_eq!(query_only, 1);
    let writer = rusqlite::Connection::open(&path).expect("writer");
    let count: i64 = writer
        .query_row("SELECT count(*) FROM items", [], |row| row.get(0))
        .expect("count");
    assert_eq!(count, 0);
}

#[test]
fn a_missing_file_is_an_error_and_is_not_created() {
    let dir = tempfile::TempDir::new().expect("temp dir");
    let path = dir.path().join("absent.db");
    assert!(connect(&path).is_err());
    assert!(!path.exists());
}

#[test]
fn rows_come_in_pages_of_200_with_a_shorter_last_page() {
    let dir = tempfile::TempDir::new().expect("temp dir");
    let path = shop(&dir);
    add_rows(&path, 450);
    let conn = connect(&path).expect("open");
    let big = table_ref("main", "big");

    let first = rows(&conn, &big, 0).expect("first page");
    assert_eq!(first.rows.len(), 200);
    assert_eq!(first.end, PageEnd::More);
    assert_eq!(first.rows[0][0], "1");

    let second = rows(&conn, &big, 200).expect("second page");
    assert_eq!(second.rows.len(), 200);
    assert_eq!(second.end, PageEnd::More);
    assert_eq!(second.rows[0][0], "201");

    let last = rows(&conn, &big, 400).expect("last page");
    assert_eq!(last.rows.len(), 50);
    assert_eq!(last.end, PageEnd::Last);
    assert_eq!(last.rows[49][0], "450");
    assert_eq!(last.columns, vec!["n".to_owned()]);
}

#[test]
fn an_odd_table_name_reads_its_columns_and_its_rows() {
    let dir = tempfile::TempDir::new().expect("temp dir");
    let conn = connect(&shop(&dir)).expect("open");
    let odd = table_ref("main", "odd\"name.v 1");
    let found = columns(&conn, &odd).expect("columns");
    assert_eq!(found.len(), 1);
    assert_eq!(found[0].name, "a \"b\"");
    let page = rows(&conn, &odd, 0).expect("rows");
    assert_eq!(page.columns, vec!["a \"b\"".to_owned()]);
    assert_eq!(page.rows, Vec::<Vec<String>>::new());
    assert_eq!(page.end, PageEnd::Last);
}

#[test]
fn every_kind_of_value_is_shown_as_one_line_of_text() {
    let dir = tempfile::TempDir::new().expect("temp dir");
    let path = shop(&dir);
    rusqlite::Connection::open(&path)
        .expect("writer")
        .execute_batch(
            "CREATE TABLE kinds (i INTEGER, r REAL, t TEXT, b BLOB, n TEXT);
             INSERT INTO kinds VALUES (7, 1.5, 'tab' || char(9) || 'end', x'00ff', NULL);",
        )
        .expect("values");
    let conn = connect(&path).expect("open");
    let page = rows(&conn, &table_ref("main", "kinds"), 0).expect("rows");
    assert_eq!(page.columns, ["i", "r", "t", "b", "n"].map(str::to_owned));
    assert_eq!(
        page.rows,
        vec![["7", "1.5", "tab·end", "<2 bytes>", "NULL"].map(str::to_owned)]
    );
}
