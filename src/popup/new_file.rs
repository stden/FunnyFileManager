//! New-file input and create-file acceptance.

use crate::models::{App, Overlay};
use crossterm::event::{KeyCode, KeyModifiers};

use crate::ops::handle_create_file;

pub(super) fn process(app: &mut App, code: KeyCode, modifiers: KeyModifiers) -> bool {
    let Overlay::NewFileInput(input) = &mut app.overlay else {
        return false;
    };
    match code {
        KeyCode::Esc | KeyCode::F(10) => {
            app.close_overlay();
        }
        KeyCode::Enter => {
            let filename = input.trim().to_owned();
            app.close_overlay();
            if let Err(err) = handle_create_file(app, &filename) {
                app.set_status(crate::status::Status::CreateFileFailed {
                    error: err.to_string(),
                });
            }
        }
        KeyCode::Backspace => {
            input.pop();
        }
        KeyCode::Char(ch)
            if !modifiers.contains(KeyModifiers::CONTROL)
                && !modifiers.contains(KeyModifiers::ALT) =>
        {
            input.push(ch);
        }
        _ => {}
    }
    true
}
