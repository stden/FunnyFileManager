//! Purpose: One open connection to a database, whichever driver it uses. Each
//! request opens its own, asks its questions and drops it, so no connection
//! outlives the answer it gave.

use super::DbError;
use crate::models::database::{
    Column, ConnectionSpec, Driver, Name, Page, PageEnd, Schema, Table, TableRef, PAGE_ROWS,
};

pub(super) enum Session {
    Sqlite(rusqlite::Connection),
    Postgres(postgres::Client),
}

impl Session {
    pub(super) fn open(spec: &ConnectionSpec) -> Result<Self, DbError> {
        match &spec.driver {
            Driver::Sqlite { path } => super::sqlite::connect(path).map(Self::Sqlite),
            Driver::Postgres(endpoint) => {
                let password = super::password::resolve(endpoint, |name| std::env::var(name).ok())?;
                super::postgres::connect(endpoint, password.as_deref()).map(Self::Postgres)
            }
        }
    }

    /// Every schema with its tables and views.
    pub(super) fn schemas(&mut self) -> Result<Vec<Schema>, DbError> {
        let names = match self {
            Self::Sqlite(connection) => super::sqlite::schemas(connection)?,
            Self::Postgres(client) => super::postgres::schemas(client)?,
        };
        let mut schemas = Vec::with_capacity(names.len());
        for name in names {
            let tables = self.tables(&name)?;
            schemas.push(Schema { name, tables });
        }
        Ok(schemas)
    }

    fn tables(&mut self, schema: &Name) -> Result<Vec<Table>, DbError> {
        match self {
            Self::Sqlite(connection) => super::sqlite::tables(connection, schema),
            Self::Postgres(client) => super::postgres::tables(client, schema),
        }
    }

    pub(super) fn columns(&mut self, table: &TableRef) -> Result<Vec<Column>, DbError> {
        match self {
            Self::Sqlite(connection) => super::sqlite::columns(connection, table),
            Self::Postgres(client) => super::postgres::columns(client, table),
        }
    }

    pub(super) fn rows(&mut self, table: &TableRef, offset: usize) -> Result<Page, DbError> {
        match self {
            Self::Sqlite(connection) => super::sqlite::rows(connection, table, offset),
            Self::Postgres(client) => super::postgres::rows(client, table, offset),
        }
    }
}

/// A page from one fetch that asked for a row more than a page: that extra
/// row is dropped and says a further page exists.
pub(super) fn page_of(columns: Vec<String>, mut rows: Vec<Vec<String>>) -> Page {
    let end = if rows.len() > PAGE_ROWS {
        rows.truncate(PAGE_ROWS);
        PageEnd::More
    } else {
        PageEnd::Last
    };
    Page { columns, rows, end }
}

/// `text` with control characters shown as a dot, so a cell stays on one line.
pub(super) fn printable(text: &str) -> String {
    text.chars()
        .map(|character| {
            if character.is_control() {
                '·'
            } else {
                character
            }
        })
        .collect()
}
