//! Purpose: the Language servers form's keys — walking and pressing its controls, typing into a field, and the suggestion list under it.

use crossterm::event::{KeyCode, KeyModifiers};

use super::{editor, requests_list, set_mode};
use crate::models::lsp_editor::{Direction, FormField, ListFocus, Mode};
use crate::models::{App, LspEditor};
use crate::ops::lsp_editor::{self as ops, Trigger};

/// The form, no list open.
pub(super) fn form(app: &mut App, code: KeyCode, modifiers: KeyModifiers) {
    if requests_list(code, modifiers) {
        ops::refresh_completion(app, Trigger::Requested);
        return;
    }
    let focus = focus(app);
    match code {
        KeyCode::Tab | KeyCode::Down => step_focus(app, Direction::Next),
        KeyCode::BackTab | KeyCode::Up => step_focus(app, Direction::Previous),
        KeyCode::Enter => press(app, focus),
        KeyCode::Esc => set_mode(app, Mode::Browse),
        KeyCode::Char(' ') if focus == Some(FormField::Enabled) => toggle_enabled(app),
        KeyCode::Char(' ') if matches!(focus, Some(FormField::Ok | FormField::Cancel)) => {
            press(app, focus);
        }
        _ => type_key(app, code, modifiers),
    }
}

fn focus(app: &mut App) -> Option<FormField> {
    editor(app)?.form_mut().map(|form| form.focus())
}

fn step_focus(app: &mut App, direction: Direction) {
    if let Some(form) = editor(app).and_then(LspEditor::form_mut) {
        form.step_focus(direction);
    }
}

fn toggle_enabled(app: &mut App) {
    if let Some(form) = editor(app).and_then(LspEditor::form_mut) {
        form.toggle_enabled();
    }
}

/// Enter: OK, or Cancel when Cancel has the cursor.
fn press(app: &mut App, focus: Option<FormField>) {
    if focus == Some(FormField::Cancel) {
        set_mode(app, Mode::Browse);
    } else {
        ops::accept_form(app);
    }
}

/// A key for the focused text field. An edit refilters an open list or,
/// with `AutoComplete` on, opens one; moving the caret closes the list, so a
/// stale list cannot be accepted.
fn type_key(app: &mut App, code: KeyCode, modifiers: KeyModifiers) {
    let Some(form) = editor(app).and_then(LspEditor::form_mut) else {
        return;
    };
    let Some(line) = form.focused_line_mut() else {
        return;
    };
    let before = line.text().to_owned();
    crate::popup::menu_dialog::edit(line, code, modifiers);
    let edited = line.text() != before;
    match code {
        KeyCode::Left | KeyCode::Right | KeyCode::Home | KeyCode::End => {
            form.set_completion(None);
        }
        _ if edited => ops::refresh_completion(app, Trigger::Typed),
        _ => {}
    }
}

/// The form with a suggestion list open under its field.
pub(super) fn list(app: &mut App, code: KeyCode, modifiers: KeyModifiers, list_focus: ListFocus) {
    if requests_list(code, modifiers) {
        ops::refresh_completion(app, Trigger::Requested);
        return;
    }
    match code {
        KeyCode::Down => step_list(app, Direction::Next),
        KeyCode::Up => step_list(app, Direction::Previous),
        KeyCode::Enter if list_focus == ListFocus::List => ops::accept_suggestion(app),
        KeyCode::Enter => {
            close_list(app);
            let field = focus(app);
            press(app, field);
        }
        KeyCode::Esc => close_list(app),
        KeyCode::Tab => step_focus(app, Direction::Next),
        KeyCode::BackTab => step_focus(app, Direction::Previous),
        _ => type_key(app, code, modifiers),
    }
}

fn step_list(app: &mut App, direction: Direction) {
    let form = editor(app).and_then(LspEditor::form_mut);
    if let Some(list) = form.and_then(|form| form.completion_mut()) {
        list.step(direction);
    }
}

fn close_list(app: &mut App) {
    if let Some(form) = editor(app).and_then(LspEditor::form_mut) {
        form.set_completion(None);
    }
}
