//! Purpose: What F2 writes — the dialog's changes, saved into the settings
//! file only while its `lsp` still holds the servers the dialog opened on.

use std::{io, path::Path};

use serde_norway::{Mapping, Value};

use super::{apply, diff, Change};
use crate::config::{document, LspSettings, Settings, LSP_KEY};

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
pub(super) fn lsp_mapping(document: &mut Mapping) -> io::Result<&mut Mapping> {
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
