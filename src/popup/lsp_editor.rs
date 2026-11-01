//! Purpose: The Language servers dialog's keys, one function per mode.
//!
//! Every key reaches this dialog while it is open, so none of them can clash
//! with a panel or command-line binding. The list's and the view's keys are
//! the keymap's `language_servers` context (`App.keymap`), looked up exactly
//! by `action::lsp_editor`; a key nothing is bound to is swallowed. The form
//! and the exit question keep the dialog protocol here.

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use super::editor::{exit_key, ExitKey};
use crate::action::lsp_editor::{resolve, LspListCommand};
use crate::models::lsp_editor::{FormTarget, ListFocus, Mode, Step};
use crate::models::{App, ExitChoice, LspEditor, Overlay};
use crate::ops::lsp_editor::{self as ops, SaveOutcome};
use crate::scroll::ScrollTop;

mod form;

use form::{form, list};

/// Which key map applies now.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Keys {
    Browse,
    View,
    Form,
    /// The form, with a suggestion list open under its field.
    List(ListFocus),
    Exit(ExitChoice),
}

pub(super) fn process(app: &mut App, code: KeyCode, modifiers: KeyModifiers) -> bool {
    let Some(editor) = editor(app) else {
        return false;
    };
    // A notice answers the key that set it. Kept longer, a "Saved to …" would
    // stand over changes made after the save.
    editor.clear_notice();
    let keys = match editor.mode() {
        Mode::Browse => Keys::Browse,
        Mode::View(_) => Keys::View,
        Mode::Form(form) => form
            .completion()
            .map_or(Keys::Form, |list| Keys::List(list.focus())),
        Mode::ConfirmExit(choice) => Keys::Exit(*choice),
    };
    let command = resolve(&app.keymap.language_servers, KeyEvent::new(code, modifiers));
    match (keys, command) {
        (Keys::Browse | Keys::View, Some(command)) => apply(app, command),
        (Keys::Browse | Keys::View, None) => {}
        (Keys::Form, _) => form(app, code, modifiers),
        (Keys::List(focus), _) => list(app, code, modifiers, focus),
        (Keys::Exit(choice), _) => confirm_exit(app, code, choice),
    }
    true
}

/// Apply a list or view command, without taking input from forms and questions.
pub fn apply(app: &mut App, command: LspListCommand) {
    let Some(editor) = editor(app) else {
        return;
    };
    let viewing = match editor.mode() {
        Mode::Browse => false,
        Mode::View(_) => true,
        Mode::Form(_) | Mode::ConfirmExit(_) => return,
    };
    editor.clear_notice();
    if viewing {
        view(app, command);
    } else {
        browse(app, command);
    }
}

fn editor(app: &mut App) -> Option<&mut LspEditor> {
    match &mut app.overlay {
        Overlay::LspEditor(editor) => Some(editor),
        _ => None,
    }
}

fn set_mode(app: &mut App, mode: Mode) {
    if let Some(editor) = editor(app) {
        editor.set_mode(mode);
    }
}

fn move_selection(app: &mut App, step: Step) {
    if let Some(editor) = editor(app) {
        editor.move_selection(step);
    }
}

/// The server list.
fn browse(app: &mut App, command: LspListCommand) {
    match command {
        LspListCommand::Move(step) => move_selection(app, step),
        LspListCommand::Edit => edit_selected(app),
        LspListCommand::ToggleEnabled => ops::toggle(app),
        LspListCommand::Add => ops::open_form(app, &FormTarget::New),
        LspListCommand::Remove => ops::delete_selected(app),
        LspListCommand::Save => {
            ops::save(app);
        }
        LspListCommand::View => set_mode(app, Mode::View(ScrollTop::TOP)),
        LspListCommand::Recheck => ops::recheck(app),
        LspListCommand::Close => close(app),
    }
}

fn edit_selected(app: &mut App) {
    let Some(language) = editor(app).and_then(|editor| editor.selected().cloned()) else {
        return;
    };
    ops::open_form(app, &FormTarget::Existing(language));
}

/// `close`: at once when nothing changed, else ask.
fn close(app: &mut App) {
    if editor(app).is_some_and(|editor| editor.dirty()) {
        set_mode(app, Mode::ConfirmExit(ExitChoice::default()));
        // A new dialog opens centred, wherever the form was dragged.
        app.dialog_mouse.reset();
    } else {
        app.close_overlay();
    }
}

/// The view of every server: it scrolls with the list's moves, checks
/// again with `recheck` and returns to the list with `view` or `close`.
fn view(app: &mut App, command: LspListCommand) {
    match command {
        LspListCommand::Move(step) => {
            if let Some(editor) = editor(app) {
                editor.scroll_view(step);
            }
        }
        LspListCommand::Recheck => ops::recheck(app),
        LspListCommand::View | LspListCommand::Close => set_mode(app, Mode::Browse),
        // The list's own commands: nothing to do in the view.
        LspListCommand::Edit
        | LspListCommand::ToggleEnabled
        | LspListCommand::Add
        | LspListCommand::Remove
        | LspListCommand::Save => {}
    }
}

/// Ctrl+Space (the editor's key), Ctrl+Down (Far's "open this field's list")
/// or Alt+Down (the Windows combo-box key, which macOS leaves alone).
fn requests_list(code: KeyCode, modifiers: KeyModifiers) -> bool {
    match code {
        KeyCode::Char(' ') => modifiers.contains(KeyModifiers::CONTROL),
        KeyCode::Down => modifiers.intersects(KeyModifiers::CONTROL | KeyModifiers::ALT),
        _ => false,
    }
}

/// Save / Discard / Cancel, with the editor's keys.
fn confirm_exit(app: &mut App, code: KeyCode, choice: ExitChoice) {
    match exit_key(code, choice) {
        Some(ExitKey::Focus(focus)) => set_mode(app, Mode::ConfirmExit(focus)),
        Some(ExitKey::Answer(ExitChoice::Save)) => match ops::save(app) {
            SaveOutcome::Saved | SaveOutcome::Unchanged => app.close_overlay(),
            SaveOutcome::Refused => set_mode(app, Mode::Browse),
        },
        Some(ExitKey::Answer(ExitChoice::Discard)) => {
            app.close_overlay();
            app.set_status(crate::status::Status::ChangesDiscarded);
        }
        Some(ExitKey::Answer(ExitChoice::Cancel)) => set_mode(app, Mode::Browse),
        None => {}
    }
}

#[cfg(test)]
mod tests;
