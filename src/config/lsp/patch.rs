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

use std::{fmt, io, path::Path};

use serde_norway::{Mapping, Value};

use super::{LspSettings, ServerSettings};
use crate::config::{document, LanguageId, Settings, LSP_KEY};

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

/// The changes that turn `baseline` into `draft`.
#[must_use]
pub fn diff(baseline: &LspSettings, draft: &LspSettings) -> Vec<Change> {
    let catalog = LspSettings::catalog();
    let mut changes = Vec::new();
    for (key, server) in &draft.0 {
        let language = LanguageId::from_key(key);
        let fallback = catalog.0.get(key);
        // The dialog changes only owned fields, so an added server that
        // differs from the file's entry in any other field was removed and
        // added again: a new server, which a field patch would not describe.
        let edited = baseline.0.get(key).filter(|before| {
            fallback.is_some() || with_owned_fields((*before).clone(), server) == *server
        });
        match (edited, fallback) {
            (Some(before), fallback) => {
                changes.extend(field_changes(&language, before, server, fallback));
            }
            (None, None) => changes.push(Change::Add {
                server: server.clone(),
                language,
            }),
            // A catalog language is always in the table; a baseline without it
            // did not come from `LspSettings`, and nothing is written for it.
            (None, Some(_)) => {}
        }
    }
    changes.extend(
        baseline
            .0
            .keys()
            .filter(|key| !draft.0.contains_key(*key) && !catalog.0.contains_key(*key))
            .map(|key| Change::Remove {
                language: LanguageId::from_key(key),
            }),
    );
    changes
}

/// The owned fields that differ between `before` and `after`: back to the
/// catalog's value is an `Unset`, anything else a `Set`.
fn field_changes(
    language: &LanguageId,
    before: &ServerSettings,
    after: &ServerSettings,
    catalog: Option<&ServerSettings>,
) -> Vec<Change> {
    OwnedField::ALL
        .into_iter()
        .filter(|field| field.value_of(before) != field.value_of(after))
        .map(|field| {
            let value = field.value_of(after);
            let language = language.clone();
            if catalog.is_some_and(|catalog| field.value_of(catalog) == value) {
                Change::Unset { language, field }
            } else {
                Change::Set {
                    language,
                    field,
                    value,
                }
            }
        })
        .collect()
}

/// Apply `changes` to the file's `lsp` mapping, keeping every other key in
/// its place.
///
/// # Errors
///
/// [`NotAMapping`] for a server entry a field must go into, and the reason
/// for an added server that does not serialize; `lsp` may then be half
/// changed, and must not be written.
pub fn apply(changes: &[Change], lsp: &mut Mapping) -> io::Result<()> {
    for change in changes {
        match change {
            Change::Set {
                language,
                field,
                value,
            } => {
                server_entry(lsp, language)?.insert(field.key().into(), value.clone());
            }
            Change::Unset { language, field } => unset(lsp, language, *field)?,
            Change::Add { language, server } => {
                let server = serde_norway::to_value(server).map_err(io::Error::other)?;
                lsp.insert(language.as_str().into(), server);
            }
            Change::Remove { language } => {
                lsp.shift_remove(language.as_str());
            }
        }
    }
    Ok(())
}

/// The mapping of `language`, created empty when the file has none.
fn server_entry<'a>(
    lsp: &'a mut Mapping,
    language: &LanguageId,
) -> Result<&'a mut Mapping, NotAMapping> {
    lsp.entry(language.as_str().into())
        .or_insert_with(|| Value::Mapping(Mapping::new()))
        .as_mapping_mut()
        .ok_or_else(|| NotAMapping(language.clone()))
}

/// Drop `field` from `language`'s mapping, and the mapping once it is empty.
fn unset(lsp: &mut Mapping, language: &LanguageId, field: OwnedField) -> Result<(), NotAMapping> {
    let Some(entry) = lsp.get_mut(language.as_str()) else {
        return Ok(());
    };
    let server = entry
        .as_mapping_mut()
        .ok_or_else(|| NotAMapping(language.clone()))?;
    server.shift_remove(field.key());
    if server.is_empty() {
        lsp.shift_remove(language.as_str());
    }
    Ok(())
}

/// What F2 writes: the servers the dialog opened on, and the changes from them.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Edit {
    baseline: LspSettings,
    changes: Vec<Change>,
}

impl Edit {
    /// The changes from `baseline` to `draft`; `None` when there are none.
    #[must_use]
    pub fn new(baseline: &LspSettings, draft: &LspSettings) -> Option<Self> {
        let changes = diff(baseline, draft);
        (!changes.is_empty()).then(|| Self {
            baseline: baseline.clone(),
            changes,
        })
    }

    /// Write the changes into the settings file at `path`; returns the
    /// servers the file then holds.
    ///
    /// # Errors
    ///
    /// Refused, with the file untouched, when its `lsp` no longer holds the
    /// servers the dialog opened on, and for every reason
    /// [`document::edit`] refuses a save. Edits outside `lsp` never block it.
    pub fn save(&self, path: &Path) -> io::Result<LspSettings> {
        document::edit(path, |document, on_disk: &Settings| {
            if on_disk.lsp != self.baseline {
                return Err(io::Error::other(format!(
                    "{LSP_KEY} in {} changed outside this dialog; close without saving \
                     and open it again to see the change",
                    document::FILE_NAME
                )));
            }
            apply(&self.changes, lsp_mapping(document)?)
        })
        .map(|settings| settings.lsp)
    }
}

/// The `lsp` mapping of `document`, created when the file has none. `lsp:`
/// or `lsp: ~` loads as the catalog, so it is replaced by an empty mapping.
fn lsp_mapping(document: &mut Mapping) -> io::Result<&mut Mapping> {
    let lsp = document
        .entry(LSP_KEY.into())
        .or_insert_with(|| Value::Mapping(Mapping::new()));
    if lsp.is_null() {
        *lsp = Value::Mapping(Mapping::new());
    }
    lsp.as_mapping_mut().ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            format!("{LSP_KEY} in {} is not a mapping", document::FILE_NAME),
        )
    })
}

#[cfg(test)]
mod tests;
