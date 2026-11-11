//! Purpose: The SQL the drivers run: quoted names and paged rows.
//!
//! Metadata comes from the server's own catalogs, with names passed as parameters
//! (PostgreSQL) or as quoted identifiers (SQLite PRAGMAs, which take no parameters).
//! Rows are read with `LIMIT` and `OFFSET` as numbers, and one extra row says
//! whether a page follows.

use super::quote::quote_identifier;
use crate::models::database::{Name, TableRef, PAGE_ROWS};

/// One row past the page, so the page can say whether more rows follow.
const FETCH: usize = PAGE_ROWS + 1;

pub(super) const SQLITE_SCHEMAS: &str = "PRAGMA database_list";

pub(super) const POSTGRES_SCHEMAS: &str = "SELECT nspname::text FROM pg_namespace \
     WHERE left(nspname, 3) <> 'pg_' AND nspname <> 'information_schema' ORDER BY nspname";

pub(super) const POSTGRES_TABLES: &str = "SELECT table_name::text, table_type::text \
     FROM information_schema.tables WHERE table_schema = $1 ORDER BY table_name";

pub(super) const POSTGRES_COLUMNS: &str = "SELECT column_name::text, data_type::text, \
     is_nullable::text FROM information_schema.columns \
     WHERE table_schema = $1 AND table_name = $2 ORDER BY ordinal_position";

pub(super) const POSTGRES_PRIMARY_KEYS: &str = "SELECT kcu.column_name::text \
     FROM information_schema.table_constraints tc \
     JOIN information_schema.key_column_usage kcu \
       ON kcu.constraint_schema = tc.constraint_schema AND kcu.constraint_name = tc.constraint_name \
     WHERE tc.constraint_type = 'PRIMARY KEY' AND tc.table_schema = $1 AND tc.table_name = $2";

pub(super) fn sqlite_tables(schema: &Name) -> String {
    format!(
        "SELECT name, type FROM {}.sqlite_master WHERE type IN ('table', 'view') \
         AND substr(name, 1, 7) <> 'sqlite_' ORDER BY name",
        quote_identifier(schema.as_str())
    )
}

pub(super) fn sqlite_columns(table: &TableRef) -> String {
    format!(
        "PRAGMA {}.table_info({})",
        quote_identifier(table.schema.as_str()),
        quote_identifier(table.name.as_str())
    )
}

/// The rows of `table` from `offset`, the `select` list written as given.
pub(super) fn select_rows(table: &TableRef, select: &str, offset: usize) -> String {
    format!(
        "SELECT {select} FROM {}.{} LIMIT {FETCH} OFFSET {offset}",
        quote_identifier(table.schema.as_str()),
        quote_identifier(table.name.as_str())
    )
}

/// The column list that reads every column as text, in the order `names` gives.
pub(super) fn text_columns(names: &[String]) -> String {
    names
        .iter()
        .map(|name| format!("{}::text", quote_identifier(name)))
        .collect::<Vec<_>>()
        .join(", ")
}

#[cfg(test)]
mod tests;
