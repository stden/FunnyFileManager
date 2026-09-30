use crate::{
    models::{App, Overlay},
    ops::path_env,
    path_env::{Mode, PathEditor},
};
use crossterm::event::{KeyCode, KeyModifiers};

#[cfg(test)]
mod tests;

pub(super) fn process(app: &mut App, code: KeyCode, modifiers: KeyModifiers) -> bool {
    let Overlay::PathEditor(editor) = &mut app.overlay else {
        return false;
    };
    let dirty = editor.dirty();
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
        Mode::Browse if code == KeyCode::Esc && !dirty => app.close_overlay(),
        Mode::Browse => browse(editor, code, modifiers),
    }
    true
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

fn browse(editor: &mut PathEditor, code: KeyCode, modifiers: KeyModifiers) {
    let reorder = modifiers.contains(KeyModifiers::CONTROL);
    match code {
        KeyCode::Up => editor.move_selection(false, reorder),
        KeyCode::Down => editor.move_selection(true, reorder),
        KeyCode::Home => editor.selected = 0,
        KeyCode::End => editor.selected = editor.entries.len().saturating_sub(1),
        KeyCode::Insert | KeyCode::F(7) => editor.edit(None),
        KeyCode::Enter | KeyCode::F(4) if !editor.entries.is_empty() => {
            editor.edit(Some(editor.selected));
        }
        KeyCode::Delete | KeyCode::F(8) => editor.remove(),
        KeyCode::F(5) => path_env::recheck(editor),
        KeyCode::F(2) => editor.mode = Mode::ConfirmSave,
        KeyCode::Esc => editor.mode = Mode::ConfirmDiscard,
        _ => {}
    }
}
