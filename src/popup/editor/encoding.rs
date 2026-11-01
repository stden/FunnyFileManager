//! Purpose: the editor's encoding dialog (Shift+F8) — step through the encodings, then Enter converts the text and R reloads the file in the choice.

use crate::encoding::TextEncoding;
use crate::models::App;
use crossterm::event::KeyCode;

pub(super) fn process_encoding(app: &mut App, code: KeyCode) {
    let Some(editor) = app.get_editor_mut() else {
        return;
    };
    let Some(choice) = editor.encoding_choice else {
        return;
    };
    match code {
        KeyCode::Esc => editor.encoding_choice = None,
        KeyCode::Up | KeyCode::Left => {
            editor.encoding_choice = cycle_encoding(choice, TextEncoding::ALL.len() - 1);
        }
        KeyCode::Down | KeyCode::Right | KeyCode::Tab => {
            editor.encoding_choice = cycle_encoding(choice, 1);
        }
        KeyCode::Char('r' | 'R') => {
            editor.encoding_choice = None;
            crate::ops::reload_editor_encoding(app, choice);
        }
        KeyCode::Enter => {
            editor.encoding_choice = None;
            crate::ops::change_editor_encoding(app, choice);
        }
        _ => {}
    }
}

/// The encoding `offset` places after `choice` in the list, wrapping around.
fn cycle_encoding(choice: TextEncoding, offset: usize) -> Option<TextEncoding> {
    let choices = TextEncoding::ALL;
    let index = choices
        .iter()
        .position(|encoding| *encoding == choice)
        .unwrap_or(0);
    choices.get((index + offset) % choices.len()).copied()
}
