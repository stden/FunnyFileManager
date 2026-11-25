//! Purpose: One configured connection: its name, its driver and where it lives.
//! No password is held here, only the name of the environment variable that
//! holds it, which the driver reads when it connects.

use std::path::PathBuf;

use super::Name;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ConnectionSpec {
    pub(crate) name: Name,
    pub(crate) driver: Driver,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Driver {
    Sqlite { path: PathBuf },
    Postgres(PostgresEndpoint),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PostgresEndpoint {
    pub(crate) host: String,
    pub(crate) port: u16,
    pub(crate) database: String,
    pub(crate) user: String,
    /// The environment variable that holds the password, read at connect time.
    pub(crate) password_env: Option<String>,
}

impl Driver {
    pub(crate) const fn kind_label(&self) -> &'static str {
        match self {
            Self::Sqlite { .. } => "sqlite",
            Self::Postgres(_) => "postgres",
        }
    }
}
