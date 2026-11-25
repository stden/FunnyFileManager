//! Purpose: What the database tests share: names, columns, pages, an SQLite
//! file to read and an endpoint nothing listens on.

use std::path::{Path, PathBuf};

use crate::models::database::{
    Column, ConnectionSpec, Driver, KeyRole, Name, Nullability, Page, PageEnd, PostgresEndpoint,
    TableRef,
};

pub(super) fn name(text: &str) -> Name {
    Name::new(text).expect("a name")
}

pub(super) fn table_ref(schema: &str, table: &str) -> TableRef {
    TableRef {
        schema: name(schema),
        name: name(table),
    }
}

pub(super) fn column(
    name: &str,
    data_type: &str,
    key: KeyRole,
    nullability: Nullability,
) -> Column {
    Column {
        name: name.to_owned(),
        data_type: data_type.to_owned(),
        key,
        nullability,
    }
}

/// The last page of the `items` table of [`shop`]: its three columns and `rows`.
pub(super) fn items_page(rows: &[&[&str]]) -> Page {
    Page {
        columns: ["id", "label", "note"].map(str::to_owned).to_vec(),
        rows: rows
            .iter()
            .map(|row| row.iter().map(|cell| (*cell).to_owned()).collect())
            .collect(),
        end: PageEnd::Last,
    }
}

pub(super) fn sqlite_spec(path: PathBuf) -> ConnectionSpec {
    ConnectionSpec {
        name: name("shop"),
        driver: Driver::Sqlite { path },
    }
}

/// An SQLite file in `dir` with a table of two rows, one holding a NULL, and a
/// view over it.
pub(super) fn shop(dir: &Path) -> ConnectionSpec {
    let path = dir.join("shop.db");
    rusqlite::Connection::open(&path)
        .expect("create the database")
        .execute_batch(
            "CREATE TABLE items (id INTEGER PRIMARY KEY, label TEXT NOT NULL, note TEXT);
             INSERT INTO items VALUES (1, 'apple', NULL), (2, 'pear', 'ripe');
             CREATE VIEW labels AS SELECT label FROM items;",
        )
        .expect("schema");
    sqlite_spec(path)
}

/// A TCP endpoint that refuses connections at once: nothing listens on port 1.
pub(super) fn closed_port() -> PostgresEndpoint {
    PostgresEndpoint {
        host: "127.0.0.1".to_owned(),
        port: 1,
        database: "postgres".to_owned(),
        user: "postgres".to_owned(),
        password_env: None,
    }
}
