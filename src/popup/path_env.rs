//! Purpose: The PATH editor's keys — the browse commands `action::path_env`
//! resolves from the keymap's `path_editor` bindings, applied here, and the
//! dialog protocol of its entry field and its two questions.

use crate::{
    action::path_env::{resolve, PathCommand},
    models::{App, Overlay},
    ops::path_env,
    path_env::{Mode, PathEditor},
};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

#[cfg(test)]
mod tests;
#[cfg(test)]
mod tests_commands;
#[cfg(test)]
mod tests_keymap;

pub(super) fn process(app: &mut App, code: KeyCode, modifiers: KeyModifiers) -> bool {
    let Overlay::PathEditor(editor) = &mut app.overlay else {
        return false;
    };
    match &mut editor.mode {
        Mode::Edit { input, .. } => match code {
            KeyCode::Esc => editor.mode = Mode::Browse,
            KeyCode::Enter => path_env::accept_entry(editor),
            _ => {
                super::text_field::edit(input, code, modifiers, &mut app.clipboard);
            }
        },
        Mode::ConfirmSave => confirm_save(editor, code),
        Mode::ConfirmDiscard => match code {
            KeyCode::Enter => app.close_overlay(),
            KeyCode::Esc => editor.mode = Mode::Browse,
            _ => {}
        },
        Mode::Browse => {
            if let Some(command) = resolve(&app.keymap.path_editor, KeyEvent::new(code, modifiers))
            {
                apply(app, command);
            }
        }
    }
    true
}

/// Apply a browse command, leaving entry fields and questions to their own input.
pub fn apply(app: &mut App, command: PathCommand) {
    let Overlay::PathEditor(editor) = &mut app.overlay else {
        return;
    };
    if !matches!(editor.mode, Mode::Browse) {
        return;
    }
    if command == PathCommand::Close && !editor.dirty() {
        app.close_overlay();
    } else {
        browse(editor, command);
    }
}

fn confirm_save(editor: &mut PathEditor, code: KeyCode) {
    match code {
        KeyCode::Esc => editor.mode = Mode::Browse,
        KeyCode::Enter => {
            if let Err(error) = path_env::save(editor) {
                editor.message = error.to_string();
                editor.mode = Mode::Browse;
            }
        }
        _ => {}
    }
}

/// The list, browsed: `Close` reaches here only with unsaved changes.
fn browse(editor: &mut PathEditor, command: PathCommand) {
    match command {
        PathCommand::CursorUp => editor.move_selection(false, false),
        PathCommand::CursorDown => editor.move_selection(true, false),
        PathCommand::MoveEntryUp => editor.move_selection(false, true),
        PathCommand::MoveEntryDown => editor.move_selection(true, true),
        PathCommand::CursorHome => editor.selected = 0,
        PathCommand::CursorEnd => editor.selected = editor.entries.len().saturating_sub(1),
        PathCommand::Add => editor.edit(None),
        PathCommand::Edit => {
            if !editor.entries.is_empty() {
                editor.edit(Some(editor.selected));
            }
        }
        PathCommand::Remove => editor.remove(),
        PathCommand::Recheck => path_env::recheck(editor),
        PathCommand::Save => editor.mode = Mode::ConfirmSave,
        PathCommand::Close => editor.mode = Mode::ConfirmDiscard,
    }
}
