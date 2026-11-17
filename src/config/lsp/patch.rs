//! Purpose: Turn the Language servers dialog's edits into the smallest change
//! to the `lsp` mapping of `settings.yaml`, and write that change.
//!
//! A save writes only the four fields the dialog owns, and only those that
//! changed. A catalog server whose field returns to the catalog's value loses
//! that field from the file instead of pinning it, so a partial override stays
//! partial and a catalog upgrade keeps reaching the user. `root_markers`,
//! `install` (an explicit `null` included) and keys this version does not know
//! are never touched, except in an added server the user removed and added
//! again: that entry is replaced whole, as the new server the draft holds.

use std::{fmt, io};

use serde_norway::Value;

use super::ServerSettings;
use crate::config::{document, LanguageId, LSP_KEY};

mod compare;
mod mapping;
mod save;

pub use compare::diff;
pub use mapping::apply;
pub use save::Edit;

/// A field of a server the dialog edits.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OwnedField {
    Enabled,
    Command,
    Args,
    Extensions,
}

impl OwnedField {
    pub const ALL: [Self; 4] = [Self::Enabled, Self::Command, Self::Args, Self::Extensions];

    /// The field's key in the settings file, which is its serde name.
    #[must_use]
    pub const fn key(self) -> &'static str {
        match self {
            Self::Enabled => "enabled",
            Self::Command => "command",
            Self::Args => "args",
            Self::Extensions => "extensions",
        }
    }

    /// The field's value in `server`, as the file holds it.
    #[must_use]
    pub fn value_of(self, server: &ServerSettings) -> Value {
        match self {
            Self::Enabled => Value::Bool(server.enabled.into()),
            Self::Command => Value::String(server.command.clone()),
            Self::Args => strings(&server.args),
            Self::Extensions => strings(&server.extensions),
        }
    }

    /// Set this field of `server` to its value in `from`.
    fn copy(self, from: &ServerSettings, server: &mut ServerSettings) {
        match self {
            Self::Enabled => server.enabled = from.enabled,
            Self::Command => server.command.clone_from(&from.command),
            Self::Args => server.args.clone_from(&from.args),
            Self::Extensions => server.extensions.clone_from(&from.extensions),
        }
    }
}

fn strings(values: &[String]) -> Value {
    Value::Sequence(values.iter().cloned().map(Value::String).collect())
}

/// `server` with the fields the dialog owns taken from `from`, and every
/// other field kept: what an edit through the dialog makes of it.
#[must_use]
pub fn with_owned_fields(mut server: ServerSettings, from: &ServerSettings) -> ServerSettings {
    for field in OwnedField::ALL {
        field.copy(from, &mut server);
    }
    server
}

/// One edit to the `lsp` mapping.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Change {
    /// Write one field of one server.
    Set {
        language: LanguageId,
        field: OwnedField,
        value: Value,
    },
    /// Drop one field, so the catalog's value applies again.
    Unset {
        language: LanguageId,
        field: OwnedField,
    },
    /// A server the catalog does not know, written in full: a new one, or
    /// one removed and added again, which replaces the file's entry.
    Add {
        language: LanguageId,
        server: ServerSettings,
    },
    /// Drop a server the catalog does not know.
    Remove { language: LanguageId },
}

/// A server entry in the file that is not a mapping, so no field can be set
/// in it. Such a file does not parse, so a save that reaches here found a
/// file that changed underneath it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NotAMapping(pub LanguageId);

impl fmt::Display for NotAMapping {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{LSP_KEY}.{} in {} is not a mapping",
            self.0,
            document::FILE_NAME
        )
    }
}

impl From<NotAMapping> for io::Error {
    fn from(entry: NotAMapping) -> Self {
        Self::new(io::ErrorKind::InvalidData, entry.to_string())
    }
}

#[cfg(test)]
mod tests;
