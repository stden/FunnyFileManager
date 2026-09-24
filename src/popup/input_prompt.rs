//! Purpose: Keys of the one-line input dialog: editing the field, the two
//! buttons, and Enter — which hands the answer to `ops::prompts`.

use crossterm::event::{KeyCode, KeyModifiers};

use crate::menu::SettingsPage;
use crate::models::{App, Overlay};

pub(super) fn process(app: &mut App, code: KeyCode, modifiers: KeyModifiers) -> bool {
    let Overlay::InputPrompt(prompt) = &mut app.overlay else {
        return false;
    };
    match code {
        // The option every F9 prompt honours: with Escape turned off in the
        // dialog settings, only F10 (and Cancel) take the dialog down.
        KeyCode::Esc if !app.settings.enabled(SettingsPage::Dialog) => {}
        KeyCode::Esc | KeyCode::F(10) => app.close_overlay(),
        // Two buttons, so every key that moves between controls means "the
        // other one".
        KeyCode::Tab | KeyCode::BackTab | KeyCode::Up | KeyCode::Down => prompt.toggle_button(),
        KeyCode::Enter => submit(app),
        _ => super::menu_dialog::edit(prompt.input_mut(), code, modifiers),
    }
    true
}

/// Enter: the dialog closes first, so an answer that opens a window of its
/// own is not closed behind it. Cancel does nothing more.
fn submit(app: &mut App) {
    if let Overlay::InputPrompt(prompt) = std::mem::take(&mut app.overlay) {
        if !prompt.on_cancel() {
            let (purpose, typed) = prompt.into_answer();
            crate::ops::prompts::submit_input(app, purpose, &typed);
        }
    }
}

#[cfg(test)]
mod tests;
