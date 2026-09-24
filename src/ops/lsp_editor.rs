//! Purpose: The Language servers dialog's effects — opening it on what
//! `settings.yaml` holds, checking what each server would start, and writing
//! the draft back through the shared settings document.
//!
//! Opening never installs or starts anything and never waits for the `PATH`
//! scan. No server runs while the dialog is open (overlays do not stack), so a
//! save only has to update `app.config.lsp` for the next editor start.

use std::{
    io,
    path::{Path, PathBuf},
};

use crate::config::{patch, LanguageId, LspSettings};
use crate::lsp::Probe;
use crate::models::lsp_editor::{Access, FormTarget, Impact, Notice, Origin, ReadOnlyReason};
use crate::models::{App, LspEditor, Overlay};

mod completion;
pub use completion::{accept_suggestion, refresh_completion, Trigger};

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

/// F9 → Commands → Language servers, and the `lsp` command.
pub fn open(app: &mut App) {
    let (access, loaded) = load(app);
    let mut editor = LspEditor::new(access, loaded);
    let directory = app.get_active_panel_ref().path.clone();
    check_all(&mut editor, &directory, &app.live.lsp_probe);
    app.live.lsp_probe.warm();
    app.set_overlay(Overlay::LspEditor(Box::new(editor)));
}

/// What the dialog opens on: the servers the settings file holds when it
/// parses — `settings.yaml`, or the legacy `settings.json` while there is
/// none, the catalog while there is neither; F2 always writes `settings.yaml`.
/// Otherwise the servers FFM would start now, read-only.
fn load(app: &App) -> (Access, LspSettings) {
    let Some(path) = app.settings.persistence.file() else {
        return read_only(app, ReadOnlyReason::NoSettingsFile);
    };
    match crate::config::Settings::load_from(path) {
        Ok(settings) => (
            Access::Writable {
                path: path.to_owned(),
            },
            settings.lsp,
        ),
        Err(error) => read_only(
            app,
            ReadOnlyReason::Document {
                path: path.to_owned(),
                reason: error.to_string(),
            },
        ),
    }
}

fn read_only(app: &App, reason: ReadOnlyReason) -> (Access, LspSettings) {
    (Access::ReadOnly(reason), app.config.lsp.clone())
}

/// Check what every server in the draft would start.
fn check_all(editor: &mut LspEditor, directory: &Path, probe: &Probe) {
    let found: Vec<_> = editor
        .draft()
        .0
        .iter()
        .map(|(language, server)| {
            let availability = crate::lsp::availability(language, server, directory, probe);
            (LanguageId::from_key(language), availability)
        })
        .collect();
    for (language, availability) in found {
        editor.set_availability(language, availability);
    }
}

/// Check what one server would start.
fn check_one(editor: &mut LspEditor, language: LanguageId, directory: &Path, probe: &Probe) {
    let Some(server) = editor.draft().0.get(language.as_str()) else {
        return;
    };
    let availability = crate::lsp::availability(language.as_str(), server, directory, probe);
    editor.set_availability(language, availability);
}

/// The open dialog, the active panel's directory and the probe, borrowed
/// together.
fn parts(app: &mut App) -> Option<(&mut LspEditor, PathBuf, &Probe)> {
    let directory = app.get_active_panel_ref().path.clone();
    let Overlay::LspEditor(editor) = &mut app.overlay else {
        return None;
    };
    Some((editor, directory, &app.live.lsp_probe))
}

/// F5: check every server again.
pub fn recheck(app: &mut App) {
    let Some((editor, directory, probe)) = parts(app) else {
        return;
    };
    check_all(editor, &directory, probe);
    editor.set_notice(Notice::Info("Checked every server again.".to_owned()));
}

/// Space: turn the selected server on or off.
pub fn toggle(app: &mut App) {
    let Some((editor, directory, probe)) = parts(app) else {
        return;
    };
    if let Some(refusal) = editor.read_only_notice() {
        editor.set_notice(refusal);
        return;
    }
    if let Some(language) = editor.toggle_selected() {
        check_one(editor, language, &directory, probe);
    }
}

/// Del: reset a catalog server to the catalog, or remove an added one.
pub fn delete_selected(app: &mut App) {
    let Some((editor, directory, probe)) = parts(app) else {
        return;
    };
    if let Some(refusal) = editor.read_only_notice() {
        editor.set_notice(refusal);
        return;
    }
    let Some((language, origin)) = editor.delete_selected() else {
        return;
    };
    let said = match origin {
        Origin::Catalog => format!("{language}: reset to the catalog."),
        Origin::Added => format!("{language}: removed."),
    };
    editor.set_notice(Notice::Info(said));
    if origin == Origin::Catalog {
        check_one(editor, language, &directory, probe);
    }
}

/// Enter / F4 on a server, Ins / F7 for a new one: open the form.
pub fn open_form(app: &mut App, target: &FormTarget) {
    let Overlay::LspEditor(editor) = &mut app.overlay else {
        return;
    };
    if let Some(refusal) = editor.read_only_notice() {
        editor.set_notice(refusal);
        return;
    }
    match editor.open_form(target) {
        // A new dialog opens centred, wherever the last form was dragged.
        Ok(()) => app.dialog_mouse.reset(),
        Err(notice) => editor.set_notice(notice),
    }
}

/// OK: the form's values into the draft, and that server checked again.
pub fn accept_form(app: &mut App) {
    let Some((editor, directory, probe)) = parts(app) else {
        return;
    };
    if let Some(language) = editor.accept_form() {
        check_one(editor, language, &directory, probe);
    }
}

/// F2: write what changed to `settings.yaml`.
pub fn save(app: &mut App) -> SaveOutcome {
    save_with(app, |path, edit| edit.save(path))
}

/// [`save`] with the writer injected, so a test can make it fail.
fn save_with(
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

#[cfg(test)]
mod tests;
