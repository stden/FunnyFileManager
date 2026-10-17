//! Purpose: closing the editor — Esc on a modified buffer asks Save / Discard / Cancel, and the keys that move between and answer those buttons.

use super::save;
use crate::models::{AfterSave, App, EditState, ExitChoice};
use crossterm::event::KeyCode;

/// What a key does in a Save / Discard / Cancel question: move the focus
/// between the buttons, or answer. The editor's exit dialog and the Language
/// servers dialog ask the same question with the same keys.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::popup) enum ExitKey {
    Focus(ExitChoice),
    Answer(ExitChoice),
}

/// Arrows and Tab move the cursor between the buttons; the hotkeys act
/// directly, so a user who knows the dialog never has to look at it.
pub(in crate::popup) const fn exit_key(code: KeyCode, choice: ExitChoice) -> Option<ExitKey> {
    Some(match code {
        KeyCode::Left => ExitKey::Focus(choice.previous()),
        KeyCode::Right | KeyCode::Tab => ExitKey::Focus(choice.next()),
        KeyCode::Enter => ExitKey::Answer(choice),
        KeyCode::Char('y' | 'Y') => ExitKey::Answer(ExitChoice::Save),
        KeyCode::Char('n' | 'N') => ExitKey::Answer(ExitChoice::Discard),
        KeyCode::Char('c' | 'C') | KeyCode::Esc => ExitKey::Answer(ExitChoice::Cancel),
        _ => return None,
    })
}

pub(super) fn confirm_exit(app: &mut App, code: KeyCode, choice: ExitChoice) {
    let Some(editor) = app.get_editor_mut() else {
        return;
    };
    let acted = match exit_key(code, choice) {
        Some(ExitKey::Focus(focus)) => {
            editor.state = EditState::ConfirmingExit(focus);
            return;
        }
        Some(ExitKey::Answer(answer)) => answer,
        None => return,
    };

    match acted {
        ExitChoice::Save => save::request_save(app, AfterSave::Close),
        ExitChoice::Discard => {
            app.close_overlay();
            app.set_status(crate::status::Status::ChangesDiscarded);
        }
        ExitChoice::Cancel => editor.state = EditState::Modified,
    }
}

pub(super) fn close_editor(app: &mut App) {
    let Some(editor) = app.get_editor_mut() else {
        return;
    };
    if editor.state.is_modified() {
        editor.state = EditState::ConfirmingExit(ExitChoice::default());
    } else {
        app.close_overlay();
    }
}
