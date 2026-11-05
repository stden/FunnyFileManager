//! Purpose: The SQLite driver. A file opens read-only and then query-only, so
//! nothing run through this connection can change it. Names come from the
//! database and are quoted by `sql`.

use std::path::Path;

use rusqlite::types::ValueRef;
use rusqlite::{Connection, OpenFlags};

use super::session::{page_of, printable};
use super::{sql, DbError};
use crate::models::database::{
    Column, KeyRole, Name, Nullability, Page, Table, TableKind, TableRef,
};

#[cfg(test)]
mod tests;

pub(super) fn connect(path: &Path) -> Result<Connection, DbError> {
    let flags = OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX;
    let connection = Connection::open_with_flags(path, flags).map_err(|error| driver(&error))?;
    connection
        .execute_batch("PRAGMA query_only = ON;")
        .map_err(|error| driver(&error))?;
    Ok(connection)
}

/// The schemas the connection sees: `main`, then any attached database.
pub(super) fn schemas(connection: &Connection) -> Result<Vec<Name>, DbError> {
    let mut statement = connection
        .prepare(sql::SQLITE_SCHEMAS)
        .map_err(|error| driver(&error))?;
    let rows = statement
        .query_map([], |row| row.get::<_, String>(1))
        .map_err(|error| driver(&error))?;
    Ok(collect(rows)?.into_iter().filter_map(Name::new).collect())
}

pub(super) fn tables(connection: &Connection, schema: &Name) -> Result<Vec<Table>, DbError> {
    let mut statement = connection
        .prepare(&sql::sqlite_tables(schema))
        .map_err(|error| driver(&error))?;
    let rows = statement
        .query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })
        .map_err(|error| driver(&error))?;
    Ok(collect(rows)?
        .into_iter()
        .filter_map(|(name, kind)| {
            Some(Table {
                name: Name::new(name)?,
                kind: if kind == "view" {
                    TableKind::View
                } else {
                    TableKind::Table
                },
            })
        })
        .collect())
}

/// `PRAGMA table_info` gives, per column: name, declared type, NOT NULL, and
/// the 1-based position in the primary key (0 when it is not part of it).
pub(super) fn columns(connection: &Connection, table: &TableRef) -> Result<Vec<Column>, DbError> {
    let mut statement = connection
        .prepare(&sql::sqlite_columns(table))
        .map_err(|error| driver(&error))?;
    let rows = statement
        .query_map([], |row| {
            let primary_position: i64 = row.get(5)?;
            let not_null: i64 = row.get(3)?;
            Ok(Column {
                name: row.get(1)?,
                data_type: row.get(2)?,
                key: if primary_position > 0 {
                    KeyRole::Primary
                } else {
                    KeyRole::Plain
                },
                nullability: if not_null == 0 {
                    Nullability::Nullable
                } else {
                    Nullability::NotNull
                },
            })
        })
        .map_err(|error| driver(&error))?;
    collect(rows)
}

pub(super) fn rows(
    connection: &Connection,
    table: &TableRef,
    offset: usize,
) -> Result<Page, DbError> {
    let mut statement = connection
        .prepare(&sql::select_rows(table, "*", offset))
        .map_err(|error| driver(&error))?;
    let names: Vec<String> = statement
        .column_names()
        .iter()
        .map(|name| (*name).to_owned())
        .collect();
    let mut cursor = statement.query([]).map_err(|error| driver(&error))?;
    let mut rows = Vec::new();
    while let Some(row) = cursor.next().map_err(|error| driver(&error))? {
        let mut cells = Vec::with_capacity(names.len());
        for index in 0..names.len() {
            cells.push(value_text(
                row.get_ref(index).map_err(|error| driver(&error))?,
            ));
        }
        rows.push(cells);
    }
    Ok(page_of(names, rows))
}

fn value_text(value: ValueRef<'_>) -> String {
    match value {
        ValueRef::Null => "NULL".to_owned(),
        ValueRef::Integer(number) => number.to_string(),
        ValueRef::Real(number) => number.to_string(),
        ValueRef::Text(bytes) => printable(&String::from_utf8_lossy(bytes)),
        ValueRef::Blob(bytes) => format!("<{} bytes>", bytes.len()),
    }
}

fn collect<T>(rows: impl Iterator<Item = rusqlite::Result<T>>) -> Result<Vec<T>, DbError> {
    rows.map(|row| row.map_err(|error| driver(&error)))
        .collect()
}

fn driver(error: &rusqlite::Error) -> DbError {
    DbError::Driver(error.to_string())
}
