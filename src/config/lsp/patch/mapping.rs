//! Purpose: Apply the dialog's changes to the file's `lsp` mapping, keeping
//! every other key in its place.

use std::io;

use serde_norway::{Mapping, Value};

use super::{Change, NotAMapping, OwnedField};
use crate::config::LanguageId;

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
