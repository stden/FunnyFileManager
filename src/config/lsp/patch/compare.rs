//! Purpose: The changes that turn the servers the dialog opened on into its
//! draft — a field back at the catalog's value is unset, not pinned.

use super::{with_owned_fields, Change, OwnedField};
use crate::config::{LanguageId, LspSettings, ServerSettings};

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
