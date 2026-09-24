//! Purpose: The Language servers dialog's keys, one function per mode.
//!
//! Every key reaches this dialog while it is open, so none of them can clash
//! with a panel or command-line binding; each means what it already means in
//! FFM or Far. Keys a mode does not list are ignored.

use crossterm::event::{KeyCode, KeyModifiers};

use super::editor::{exit_key, ExitKey};
use crate::models::lsp_editor::{Direction, FormField, FormTarget, ListFocus, Mode, Step};
use crate::models::{App, ExitChoice, LspEditor, Overlay};
use crate::ops::lsp_editor::{self as ops, SaveOutcome, Trigger};
use crate::scroll::ScrollTop;

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
    match keys {
        Keys::Browse if modifiers.is_empty() => browse(app, code),
        Keys::View if modifiers.is_empty() => view(app, code),
        // No list or view key takes a modifier, and a modified key is another
        // key: Shift+F8 or Alt+F7 means something else in Far.
        Keys::Browse | Keys::View => {}
        Keys::Form => form(app, code, modifiers),
        Keys::List(focus) => list(app, code, modifiers, focus),
        Keys::Exit(choice) => confirm_exit(app, code, choice),
    }
    true
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

/// The move a navigation key makes in a list or the view.
const fn step(code: KeyCode) -> Option<Step> {
    match code {
        KeyCode::Up => Some(Step::Up),
        KeyCode::Down => Some(Step::Down),
        KeyCode::PageUp => Some(Step::PageUp),
        KeyCode::PageDown => Some(Step::PageDown),
        KeyCode::Home => Some(Step::First),
        KeyCode::End => Some(Step::Last),
        _ => None,
    }
}

/// The server list.
fn browse(app: &mut App, code: KeyCode) {
    if let (Some(step), Some(editor)) = (step(code), editor(app)) {
        editor.move_selection(step);
        return;
    }
    match code {
        KeyCode::Enter | KeyCode::F(4) => edit_selected(app),
        KeyCode::Char(' ') => ops::toggle(app),
        KeyCode::Insert | KeyCode::F(7) => ops::open_form(app, &FormTarget::New),
        KeyCode::Delete | KeyCode::F(8) => ops::delete_selected(app),
        KeyCode::F(2) => {
            ops::save(app);
        }
        KeyCode::F(3) => set_mode(app, Mode::View(ScrollTop::TOP)),
        KeyCode::F(5) => ops::recheck(app),
        KeyCode::Esc | KeyCode::F(10) => close(app),
        _ => {}
    }
}

fn edit_selected(app: &mut App) {
    let Some(language) = editor(app).and_then(|editor| editor.selected().cloned()) else {
        return;
    };
    ops::open_form(app, &FormTarget::Existing(language));
}

/// Esc / F10: close at once when nothing changed, else ask.
fn close(app: &mut App) {
    if editor(app).is_some_and(|editor| editor.dirty()) {
        set_mode(app, Mode::ConfirmExit(ExitChoice::default()));
        // A new dialog opens centred, wherever the form was dragged.
        app.dialog_mouse.reset();
    } else {
        app.close_overlay();
    }
}

/// F3: every server, scrolled.
fn view(app: &mut App, code: KeyCode) {
    if let (Some(step), Some(editor)) = (step(code), editor(app)) {
        editor.scroll_view(step);
        return;
    }
    match code {
        KeyCode::F(5) => ops::recheck(app),
        KeyCode::F(3 | 10) | KeyCode::Esc => set_mode(app, Mode::Browse),
        _ => {}
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

/// The form, no list open.
fn form(app: &mut App, code: KeyCode, modifiers: KeyModifiers) {
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
    super::menu_dialog::edit(line, code, modifiers);
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
fn list(app: &mut App, code: KeyCode, modifiers: KeyModifiers, list_focus: ListFocus) {
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
