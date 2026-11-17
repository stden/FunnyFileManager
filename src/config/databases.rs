//! Purpose: Read `databases.yaml`, the connections the database explorer offers.
//!
//! One list entry per connection. A password is never in the file: an entry names
//! the environment variable that holds it. Every refusal names the rule it breaks,
//! and never the value that broke it.

use std::fmt;
use std::io::Read;
use std::path::{Path, PathBuf};

use serde::Deserialize;

use crate::models::database::{ConnectionSpec, Driver, Name, PostgresEndpoint};

/// The file's name in the config directory.
pub(crate) const FILE_NAME: &str = "databases.yaml";
/// A connection list is a few lines per entry; a bigger file is refused unread.
const MAX_BYTES: u64 = 256 * 1024;
const DEFAULT_HOST: &str = "localhost";
const DEFAULT_PORT: u16 = 5432;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum LoadError {
    Missing,
    Unreadable,
    /// The text is not YAML, or an entry has a field its kind does not take.
    Malformed {
        line: Option<usize>,
    },
    /// An entry breaks a rule; the text is the rule.
    Invalid(&'static str),
}

impl fmt::Display for LoadError {
    fn fmt(&self, out: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Missing => out.write_str("no databases.yaml"),
            Self::Unreadable => out.write_str("databases.yaml cannot be read"),
            Self::Malformed { line: Some(line) } => write!(
                out,
                "databases.yaml line {line} is not a valid connection (a password goes in password_env, never in this file)"
            ),
            Self::Malformed { line: None } => {
                out.write_str("databases.yaml is not a list of connections")
            }
            Self::Invalid(rule) => write!(out, "databases.yaml: {rule}"),
        }
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Entry {
    name: String,
    kind: Kind,
    path: Option<PathBuf>,
    host: Option<String>,
    port: Option<u16>,
    database: Option<String>,
    user: Option<String>,
    password_env: Option<String>,
}

#[derive(Deserialize, Clone, Copy)]
#[serde(rename_all = "lowercase")]
enum Kind {
    Sqlite,
    Postgres,
}

/// The connections the file lists; a missing file is [`LoadError::Missing`].
pub(crate) fn load(path: &Path) -> Result<Vec<ConnectionSpec>, LoadError> {
    parse(&read(path)?)
}

/// The connections `text` lists, checked. An empty text lists none.
pub(crate) fn parse(text: &str) -> Result<Vec<ConnectionSpec>, LoadError> {
    if text.trim().is_empty() {
        return Ok(Vec::new());
    }
    let entries: Vec<Entry> =
        serde_norway::from_str(text).map_err(|error| LoadError::Malformed {
            line: error.location().map(|location| location.line()),
        })?;
    let specs = entries
        .into_iter()
        .map(to_spec)
        .collect::<Result<Vec<_>, _>>()?;
    if has_duplicate_name(&specs) {
        return Err(LoadError::Invalid("two connections share a name"));
    }
    Ok(specs)
}

fn read(path: &Path) -> Result<String, LoadError> {
    let file = match std::fs::File::open(path) {
        Ok(file) => file,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return Err(LoadError::Missing);
        }
        Err(_) => return Err(LoadError::Unreadable),
    };
    let mut text = String::new();
    if file
        .take(MAX_BYTES.saturating_add(1))
        .read_to_string(&mut text)
        .is_err()
    {
        return Err(LoadError::Unreadable);
    }
    if u64::try_from(text.len()).is_ok_and(|len| len > MAX_BYTES) {
        return Err(LoadError::Unreadable);
    }
    Ok(text)
}

fn to_spec(entry: Entry) -> Result<ConnectionSpec, LoadError> {
    let name =
        Name::new(entry.name.trim()).ok_or(LoadError::Invalid("a connection needs a name"))?;
    let driver = match entry.kind {
        Kind::Sqlite => Driver::Sqlite {
            path: entry
                .path
                .ok_or(LoadError::Invalid("a sqlite connection needs a path"))?,
        },
        Kind::Postgres => Driver::Postgres(postgres_endpoint(&entry)?),
    };
    Ok(ConnectionSpec { name, driver })
}

fn postgres_endpoint(entry: &Entry) -> Result<PostgresEndpoint, LoadError> {
    if entry
        .password_env
        .as_deref()
        .is_some_and(|variable| !is_variable_name(variable))
    {
        return Err(LoadError::Invalid(
            "password_env must name an environment variable",
        ));
    }
    Ok(PostgresEndpoint {
        host: entry
            .host
            .clone()
            .unwrap_or_else(|| DEFAULT_HOST.to_owned()),
        port: entry.port.unwrap_or(DEFAULT_PORT),
        database: required(
            entry.database.as_ref(),
            "a postgres connection needs a database",
        )?,
        user: required(entry.user.as_ref(), "a postgres connection needs a user")?,
        password_env: entry.password_env.clone(),
    })
}

fn required(value: Option<&String>, rule: &'static str) -> Result<String, LoadError> {
    match value {
        Some(text) if !text.trim().is_empty() => Ok(text.trim().to_owned()),
        _ => Err(LoadError::Invalid(rule)),
    }
}

fn is_variable_name(text: &str) -> bool {
    let mut chars = text.chars();
    matches!(chars.next(), Some(first) if first.is_ascii_alphabetic() || first == '_')
        && chars.all(|character| character.is_ascii_alphanumeric() || character == '_')
}

fn has_duplicate_name(specs: &[ConnectionSpec]) -> bool {
    let mut names: Vec<&Name> = specs.iter().map(|spec| &spec.name).collect();
    names.sort();
    names
        .windows(2)
        .any(|pair| matches!(pair, [first, second] if first == second))
}

#[cfg(test)]
mod tests;
