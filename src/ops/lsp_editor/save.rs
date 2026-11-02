//! Purpose: F2 in the Language servers dialog: write what changed to
//! `settings.yaml`, or say in the message row why nothing was written.

use std::{
    io,
    path::{Path, PathBuf},
};

use super::check_all;
use crate::config::{patch, LspSettings};
use crate::models::lsp_editor::{Access, Impact, Notice};
use crate::models::{App, LspEditor, Overlay};

/// What a save came to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SaveOutcome {
    /// The file was written.
    Saved,
    /// The draft held no change to write.
    Unchanged,
    /// Nothing was written; the message row says why.
    Refused,
}

/// F2: write what changed to `settings.yaml`.
pub fn save(app: &mut App) -> SaveOutcome {
    save_with(app, |path, edit| edit.save(path))
}

/// [`save`] with the writer injected, so a test can make it fail.
pub(super) fn save_with(
    app: &mut App,
    write: impl FnOnce(&Path, &patch::Edit) -> io::Result<LspSettings>,
) -> SaveOutcome {
    let directory = app.get_active_panel_ref().path.clone();
    let Overlay::LspEditor(editor) = &mut app.overlay else {
        return SaveOutcome::Refused;
    };
    let (path, edit) = match prepare(editor) {
        Ok(ready) => ready,
        Err(outcome) => return outcome,
    };
    match write(&path, &edit) {
        Ok(lsp) => {
            editor.saved(lsp.clone());
            check_all(editor, &directory, &app.live.lsp_probe);
            editor.set_notice(Notice::Info(format!(
                "Saved to {}. Changes apply the next time a file is opened in the editor.",
                path.display()
            )));
            app.config.adopt_saved_lsp(lsp);
            SaveOutcome::Saved
        }
        Err(error) => {
            editor.set_notice(Notice::Error(error.to_string()));
            SaveOutcome::Refused
        }
    }
}

/// Where to write and what, or why nothing is written.
fn prepare(editor: &mut LspEditor) -> Result<(PathBuf, patch::Edit), SaveOutcome> {
    if let Some(refusal) = editor.read_only_notice() {
        editor.set_notice(refusal);
        return Err(SaveOutcome::Refused);
    }
    // Only a read-only dialog, refused above, has no file.
    let Access::Writable { path } = editor.access() else {
        return Err(SaveOutcome::Refused);
    };
    let path = path.clone();
    let blocking = editor
        .problems()
        .into_iter()
        .find(|(_, impact)| *impact == Impact::Blocking);
    if let Some((problem, _)) = blocking {
        editor.select(problem.language());
        editor.set_notice(Notice::Error(format!("Not saved: {problem}.")));
        return Err(SaveOutcome::Refused);
    }
    let Some(edit) = patch::Edit::new(editor.baseline(), editor.draft()) else {
        editor.set_notice(Notice::Info("Nothing to save.".to_owned()));
        return Err(SaveOutcome::Unchanged);
    };
    Ok((path, edit))
}
