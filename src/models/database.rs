//! Purpose: The database explorer's state and the values it carries.
//!
//! The connections it lists, the tree of their metadata, the rows and columns
//! views, and the one request in flight. Values and transitions only: the drivers
//! that answer requests are in `ops::database`, and the job that runs them is in
//! `application::database_jobs`.

pub mod explorer;
pub mod meta;
pub mod name;
pub mod request;
pub mod spec;

pub use explorer::DatabaseExplorer;
pub(crate) use explorer::{Back, Load, Row, RowsView, TablePath, View};
pub(crate) use meta::{
    Column, KeyRole, Nullability, Page, PageEnd, Schema, Table, TableKind, TableRef, PAGE_ROWS,
};
pub(crate) use name::Name;
pub(crate) use request::{Reply, Request, RequestId, Requests, Task};
pub(crate) use spec::{ConnectionSpec, Driver, PostgresEndpoint};

#[cfg(test)]
mod tests;
