//! Purpose: F2 and the exit dialog's Save — when the buffer is written, and
//! the question asked first when another program has changed the file.
//!
//! A child of the editor controller because both of its save entry points,
//! and the answer to the question, must end in the same write.

use crossterm::event::KeyCode;

use crate::models::{AfterSave, App, EditState, OverwritePrompt};
use crate::status::Status;
use crate::utils::refresh_panels;

/// F2: save a modified buffer and keep editing.
pub(super) fn save_editor(app: &mut App) {
    let Some(editor) = app.get_editor_mut() else {
        return;
    };
    // Nothing edited is nothing to write. Rewriting anyway is not harmless:
    // it replaces the file's inode, bumps its time, and would put back
    // whatever the buffer could not represent exactly.
    if !editor.state.is_modified() {
        editor.notice = Some("No changes to save".to_owned());
        return;
    }
    request_save(app, AfterSave::Stay);
}

/// Save the buffer, then stay or close as `then` says — unless another
/// program has written the file since the editor read it. That write would
/// be lost without a trace, so the user is asked first.
pub(super) fn request_save(app: &mut App, then: AfterSave) {
    let Some(editor) = app.get_editor_mut() else {
        return;
    };
    if editor.changed_on_disk() {
        editor.state = EditState::ConfirmingOverwrite(OverwritePrompt::new(then));
        return;
    }
    write_buffer(app, then);
}

/// The answer to "the file changed on disk. Overwrite?".
///
/// Arrows and Tab move between the two buttons, Enter takes the focused one,
/// and the hotkeys answer directly.
pub(super) fn confirm_overwrite(app: &mut App, code: KeyCode, prompt: OverwritePrompt) {
    let Some(editor) = app.get_editor_mut() else {
        return;
    };
    let overwrite = match code {
        KeyCode::Left | KeyCode::Right | KeyCode::Tab | KeyCode::BackTab => {
            let answer = prompt.answer.toggled();
            editor.state = EditState::ConfirmingOverwrite(OverwritePrompt { answer, ..prompt });
            return;
        }
        KeyCode::Enter => prompt.answer.is_yes(),
        KeyCode::Char('y' | 'Y' | 'o' | 'O') => true,
        KeyCode::Char('n' | 'N' | 'c' | 'C') | KeyCode::Esc | KeyCode::F(10) => false,
        _ => return,
    };
    if overwrite {
        write_buffer(app, prompt.then);
    } else {
        editor.state = EditState::Modified;
        editor.notice = Some("Not saved: the file changed on disk".to_owned());
    }
}

/// Write the buffer, then stay or close; a failure keeps the editor open with
/// the reason in its bar.
///
/// Split out because the borrow of the editor has to end before `app` can be
/// told about the outcome.
fn write_buffer(app: &mut App, then: AfterSave) {
    let Some(editor) = app.get_editor_mut() else {
        return;
    };
    let result = editor.save();
    let path = editor.path.clone();
    match result {
        Ok(()) => {
            if then == AfterSave::Close {
                app.close_overlay();
            }
            app.set_status(Status::Saved(path));
        }
        Err(err) => {
            if let Some(editor) = app.get_editor_mut() {
                editor.state = EditState::Modified;
                // The editor covers the panel's message row, so the status
                // line alone would report the failure to a screen nobody can
                // see. The bar is the one place the user is looking.
                editor.notice = Some(format!("Cannot save: {err}"));
            }
            app.set_status(Status::SaveFailed {
                error: err.to_string(),
            });
        }
    }
    // The save's own outcome is already on screen; a panel that cannot be
    // re-read keeps its old listing until the next refresh.
    drop(refresh_panels(app));
}
