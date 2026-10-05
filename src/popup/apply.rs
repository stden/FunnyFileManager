//! Purpose: Keys for the Apply command dialog (Ctrl+G): one field, OK and Cancel,
//! and Enter handing the template to `ops::apply` for one run per entry.

use crossterm::event::{KeyCode, KeyModifiers};

use crate::menu::SettingsPage;
use crate::models::{App, Overlay};

pub(super) fn process(app: &mut App, code: KeyCode, modifiers: KeyModifiers) -> bool {
    let Overlay::ApplyCommand(prompt) = &mut app.overlay else {
        return false;
    };
    match code {
        // The same option every other F9 prompt honours: with Escape turned
        // off in the dialog settings, only F10 (and Cancel) take it down.
        KeyCode::Esc if !app.settings.enabled(SettingsPage::Dialog) => {}
        KeyCode::Esc | KeyCode::F(10) => app.close_overlay(),
        KeyCode::Tab => prompt.toggle_button(),
        KeyCode::Enter => submit(app),
        // Left / Right, Home / End, Delete and plain characters edit the
        // field, exactly as in the generic prompts; Ctrl / Alt chords are
        // not text.
        _ => {
            super::text_field::edit(prompt.input_mut(), code, modifiers, &mut app.clipboard);
        }
    }
    true
}

/// Enter: run the template, unless the cursor is on Cancel or nothing was
/// typed. A blank command is treated as Cancel rather than run: `sh -c ""`
/// would echo an empty prompt line into the transcript for every entry and
/// call that a success.
fn submit(app: &mut App) {
    let Overlay::ApplyCommand(prompt) = &app.overlay else {
        return;
    };
    let template = prompt.command().trim().to_owned();
    let cancelled = prompt.on_cancel() || template.is_empty();
    app.close_overlay();
    if !cancelled {
        crate::ops::apply::run(app, &template);
    }
}

#[cfg(test)]
mod tests;
