//! Purpose: The PostgreSQL driver: read-only sessions and text values.
//!
//! Every session is read-only and every statement is cut short after a fixed time.
//! Names come from the server's catalogs and are quoted by `sql`; values are read
//! as text, so every column type shows without a conversion per type.

use std::time::Duration;

use postgres::{Client, Config, NoTls, SimpleQueryMessage};

use super::password::redact;
use super::session::{page_of, printable};
use super::{sql, DbError};
use crate::models::database::{
    Column, KeyRole, Name, Nullability, Page, PostgresEndpoint, Table, TableKind, TableRef,
};

#[cfg(test)]
mod tests;

const CONNECT_TIMEOUT: Duration = Duration::from_secs(5);
/// Longest a single statement may run, in milliseconds.
const STATEMENT_TIMEOUT_MS: u32 = 5_000;

pub(super) fn connect(
    endpoint: &PostgresEndpoint,
    password: Option<&str>,
) -> Result<Client, DbError> {
    let mut config = Config::new();
    config
        .host(&endpoint.host)
        .port(endpoint.port)
        .dbname(&endpoint.database)
        .user(&endpoint.user)
        .connect_timeout(CONNECT_TIMEOUT);
    if let Some(secret) = password {
        config.password(secret);
    }
    let mut client = config
        .connect(NoTls)
        .map_err(|error| DbError::Driver(redact(&error.to_string(), password)))?;
    read_only(&mut client)?;
    Ok(client)
}

/// The session refuses writes, and a statement that runs too long is cancelled.
pub(super) fn read_only(client: &mut Client) -> Result<(), DbError> {
    client
        .batch_execute(&format!(
            "SET SESSION CHARACTERISTICS AS TRANSACTION READ ONLY; \
             SET statement_timeout = {STATEMENT_TIMEOUT_MS};"
        ))
        .map_err(|error| driver(&error))
}

/// The non-system schemas of the database.
pub(super) fn schemas(client: &mut Client) -> Result<Vec<Name>, DbError> {
    let mut names = Vec::new();
    for row in client
        .query(sql::POSTGRES_SCHEMAS, &[])
        .map_err(|error| driver(&error))?
    {
        let name: String = row.try_get(0).map_err(|error| driver(&error))?;
        names.extend(Name::new(name));
    }
    Ok(names)
}

pub(super) fn tables(client: &mut Client, schema: &Name) -> Result<Vec<Table>, DbError> {
    let mut tables = Vec::new();
    for row in client
        .query(sql::POSTGRES_TABLES, &[&schema.as_str()])
        .map_err(|error| driver(&error))?
    {
        let name: String = row.try_get(0).map_err(|error| driver(&error))?;
        let kind: String = row.try_get(1).map_err(|error| driver(&error))?;
        let kind = if kind == "VIEW" {
            TableKind::View
        } else {
            TableKind::Table
        };
        tables.extend(Name::new(name).map(|name| Table { name, kind }));
    }
    Ok(tables)
}

pub(super) fn columns(client: &mut Client, table: &TableRef) -> Result<Vec<Column>, DbError> {
    let schema = table.schema.as_str();
    let name = table.name.as_str();
    let primary = primary_key_columns(client, schema, name)?;
    let mut columns = Vec::new();
    for row in client
        .query(sql::POSTGRES_COLUMNS, &[&schema, &name])
        .map_err(|error| driver(&error))?
    {
        let column: String = row.try_get(0).map_err(|error| driver(&error))?;
        let data_type: String = row.try_get(1).map_err(|error| driver(&error))?;
        let nullable: String = row.try_get(2).map_err(|error| driver(&error))?;
        columns.push(Column {
            key: if primary.contains(&column) {
                KeyRole::Primary
            } else {
                KeyRole::Plain
            },
            nullability: if nullable == "YES" {
                Nullability::Nullable
            } else {
                Nullability::NotNull
            },
            name: column,
            data_type,
        });
    }
    Ok(columns)
}

fn primary_key_columns(
    client: &mut Client,
    schema: &str,
    table: &str,
) -> Result<Vec<String>, DbError> {
    let mut names = Vec::new();
    for row in client
        .query(sql::POSTGRES_PRIMARY_KEYS, &[&schema, &table])
        .map_err(|error| driver(&error))?
    {
        names.push(
            row.try_get::<_, String>(0)
                .map_err(|error| driver(&error))?,
        );
    }
    Ok(names)
}

/// One page of `table`. The server's column names come from preparing the
/// query (which works for an empty table too); the values are then fetched
/// with each column cast to text.
pub(super) fn rows(client: &mut Client, table: &TableRef, offset: usize) -> Result<Page, DbError> {
    let statement = client
        .prepare(&sql::select_rows(table, "*", offset))
        .map_err(|error| driver(&error))?;
    let names: Vec<String> = statement
        .columns()
        .iter()
        .map(|column| column.name().to_owned())
        .collect();
    let select = sql::text_columns(&names);
    let mut rows = Vec::new();
    for message in client
        .simple_query(&sql::select_rows(table, &select, offset))
        .map_err(|error| driver(&error))?
    {
        if let SimpleQueryMessage::Row(row) = message {
            rows.push(
                (0..row.len())
                    .map(|index| row.get(index).map_or_else(|| "NULL".to_owned(), printable))
                    .collect(),
            );
        }
    }
    Ok(page_of(names, rows))
}

fn driver(error: &postgres::Error) -> DbError {
    DbError::Driver(error.to_string())
}
