//! Purpose: The database explorer's effects: open the dialog, run requests.
//!
//! `open` builds the dialog from `databases.yaml`; `execute` runs one request on its
//! own connection. Drivers are in `sqlite` and `postgres`, the SQL text in `sql`,
//! and the answer is delivered to the dialog by `completion`.

pub mod completion;
pub mod error;
pub mod password;
pub mod postgres;
pub mod quote;
pub mod session;
pub mod sql;
pub mod sqlite;

use std::path::Path;

pub(crate) use completion::apply;
pub(crate) use error::DbError;
use session::Session;

use crate::config::databases::{self, LoadError};
use crate::models::database::{DatabaseExplorer, Reply, Task};
use crate::models::{App, Overlay};
use crate::status::Status;

/// Commands → Database explorer: the dialog over the connections the config lists.
pub(crate) fn open(app: &mut App) {
    let explorer = match crate::config::dir() {
        Some(dir) => explorer_from(&dir.join(databases::FILE_NAME)),
        None => DatabaseExplorer::new(
            Vec::new(),
            Some("No config directory, so no databases.yaml to read".to_owned()),
        ),
    };
    let notice = explorer.notice().map(str::to_owned);
    app.set_overlay(Overlay::Database(Box::new(explorer)));
    if let Some(text) = notice {
        app.set_status(Status::Error(text));
    }
}

fn explorer_from(path: &Path) -> DatabaseExplorer {
    match databases::load(path) {
        Ok(specs) if specs.is_empty() => DatabaseExplorer::new(
            specs,
            Some(format!("{} lists no connections", path.display())),
        ),
        Ok(specs) => DatabaseExplorer::new(specs, None),
        Err(LoadError::Missing) => DatabaseExplorer::new(
            Vec::new(),
            Some(format!(
                "No connections: create {} with one entry per database",
                path.display()
            )),
        ),
        Err(error) => {
            DatabaseExplorer::new(Vec::new(), Some(format!("{}: {error}", path.display())))
        }
    }
}

/// Run one request on its own connection. A password is read here, from the
/// variable the entry names, and goes no further than the driver.
pub(crate) fn execute(task: &Task) -> Result<Reply, DbError> {
    match task {
        Task::Schemas(spec) => Session::open(spec)?.schemas().map(Reply::Schemas),
        Task::Columns { spec, table } => Session::open(spec)?.columns(table).map(Reply::Columns),
        Task::Rows {
            spec,
            table,
            offset,
        } => Session::open(spec)?.rows(table, *offset).map(Reply::Rows),
    }
}
