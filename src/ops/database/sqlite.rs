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
    let connection = Connection::open_with_flags(path, flags)?;
    connection.execute_batch("PRAGMA query_only = ON;")?;
    Ok(connection)
}

/// The schemas the connection sees: `main`, then any attached database.
pub(super) fn schemas(connection: &Connection) -> Result<Vec<Name>, DbError> {
    let mut statement = connection.prepare(sql::SQLITE_SCHEMAS)?;
    let rows = statement.raw_query().mapped(|row| row.get::<_, String>(1));
    Ok(collect(rows)?.into_iter().filter_map(Name::new).collect())
}

pub(super) fn tables(connection: &Connection, schema: &Name) -> Result<Vec<Table>, DbError> {
    let mut statement = connection.prepare(&sql::sqlite_tables(schema))?;
    let rows = statement
        .raw_query()
        .mapped(|row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)));
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
    let mut statement = connection.prepare(&sql::sqlite_columns(table))?;
    let rows = statement.raw_query().mapped(|row| {
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
    });
    collect(rows)
}

pub(super) fn rows(
    connection: &Connection,
    table: &TableRef,
    offset: usize,
) -> Result<Page, DbError> {
    let mut statement = connection.prepare(&sql::select_rows(table, "*", offset))?;
    let names: Vec<String> = statement
        .column_names()
        .map(|name| (*name).to_owned())
        .collect();
    let mut cursor = statement.raw_query();
    let mut rows = Vec::new();
    while let Some(row) = cursor.next()? {
        let mut cells = Vec::with_capacity(names.len());
        for index in 0..names.len() {
            cells.push(value_text(row.get_ref(index)?));
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
    rows.collect::<rusqlite::Result<Vec<T>>>()
        .map_err(DbError::from)
}

impl From<rusqlite::Error> for DbError {
    fn from(error: rusqlite::Error) -> Self {
        Self::Driver(error.to_string())
    }
}
