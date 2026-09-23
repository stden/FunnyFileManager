//! Editor input, with modal search and confirmation taking priority.

use super::{process_search_prompt, repeat_search, EDITOR_PAGE};
use crate::action::editor::{self as editor_keymap, EditorCommand, EditorNavigation};
use crate::models::{App, EditState, EditorMotion, ExitChoice};
use crate::search::SearchPrompt;
use crate::utils::refresh_panels;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

/// Save the open editor and close it, or report why it could not be saved.
///
/// Split out because the borrow of the editor has to end before `app` can be
/// told about the outcome, and inlining it put a mutable borrow of `app` inside
/// one of `app.get_editor_mut()`.
fn save_and_close(app: &mut App) {
    let Some(editor) = app.get_editor_mut() else {
        return;
    };
    let result = editor.save();
    let path = editor.path.clone();
    match result {
        Ok(()) => {
            app.close_overlay();
            app.set_status(crate::status::Status::Saved(path));
            let _ = refresh_panels(app);
        }
        Err(err) => {
            if let Some(editor) = app.get_editor_mut() {
                editor.state = EditState::Modified;
                // The editor covers the panel's message row, so the status
                // line alone would report the failure to a screen nobody can
                // see. The bar is the one place the user is looking.
                editor.notice = Some(format!("Cannot save: {err}"));
            }
            app.set_status(crate::status::Status::SaveFailed {
                error: err.to_string(),
            });
        }
    }
}

/// A terminal paste is document text, never a sequence of editor shortcuts.
pub fn paste_text(app: &mut App, text: &str) {
    if let Some(editor) = app.get_editor_mut().filter(|editor| editor.accepts_text()) {
        editor.search.not_found = None;
        editor.notice = None;
        editor.insert_text(text);
    }
}

pub(super) fn process_editor_input(app: &mut App, code: KeyCode, modifiers: KeyModifiers) -> bool {
    let Some(editor) = app.get_editor_mut() else {
        return false;
    };

    if let Some(choice) = editor.state.exit_choice() {
        confirm_exit(app, code, choice);
        return true;
    }

    if editor.search.prompt.is_some() {
        return process_search_prompt(app, code);
    }
    if editor.encoding_choice.is_some() {
        process_encoding(app, code);
        return true;
    }
    if lsp::process(app, code) {
        return true;
    }
    let Some(editor) = app.get_editor_mut() else {
        return false;
    };
    // Any keystroke clears a stale "not found": it belongs to the search the
    // user has now moved on from.
    editor.search.not_found = None;
    editor.notice = None;

    let command = editor_keymap::resolve(KeyEvent::new(code, modifiers));
    apply_editor_command(app, command);
    true
}

fn confirm_exit(app: &mut App, code: KeyCode, choice: ExitChoice) {
    let Some(editor) = app.get_editor_mut() else {
        return;
    };
    // Arrows and Tab move the cursor between the buttons; the hotkeys act
    // directly, so a user who knows the dialog never has to look at it.
    let acted = match code {
        KeyCode::Left => {
            editor.state = EditState::ConfirmingExit(choice.previous());
            return;
        }
        KeyCode::Right | KeyCode::Tab => {
            editor.state = EditState::ConfirmingExit(choice.next());
            return;
        }
        KeyCode::Enter => choice,
        KeyCode::Char('y' | 'Y') => ExitChoice::Save,
        KeyCode::Char('n' | 'N') => ExitChoice::Discard,
        KeyCode::Char('c' | 'C') | KeyCode::Esc => ExitChoice::Cancel,
        _ => return,
    };

    match acted {
        ExitChoice::Save => save_and_close(app),
        ExitChoice::Discard => {
            app.close_overlay();
            app.set_status(crate::status::Status::ChangesDiscarded);
        }
        ExitChoice::Cancel => editor.state = EditState::Modified,
    }
}

/// Interpret the editor command language after modal input has been handled.
fn apply_editor_command(app: &mut App, command: EditorCommand) {
    let tab_width = app.settings.number(crate::menu::SettingsPage::Editor);
    let Some(editor) = app.get_editor_mut() else {
        return;
    };

    match command {
        EditorCommand::Move { motion, selection } => {
            editor.move_cursor(editor_motion(motion), selection);
        }
        EditorCommand::Copy => crate::ops::copy_editor_selection(app),
        EditorCommand::SelectAll => editor.select_all(),
        EditorCommand::ChooseEncoding => editor.encoding_choice = Some(editor.format.encoding),
        EditorCommand::Paste => crate::ops::paste_into_editor(app),
        EditorCommand::RepeatSearch => repeat_search(app),
        EditorCommand::Lsp(feature) => {
            editor.lsp.intent = crate::models::lsp::Intent::Query(feature);
        }
        EditorCommand::RestartLsp => editor.lsp.intent = crate::models::lsp::Intent::Restart,
        EditorCommand::Search => {
            editor.search.prompt = Some(SearchPrompt::from_last(editor.search.last.as_ref()));
        }
        EditorCommand::Close => close_editor(app),
        EditorCommand::Save => save_editor(app),
        EditorCommand::Newline => editor.insert_newline(),
        EditorCommand::Backspace => editor.backspace(),
        EditorCommand::Delete => editor.delete_forward(),
        EditorCommand::Indent => insert_indent(editor, tab_width),
        EditorCommand::Insert(ch) => editor.insert_char(ch),
        EditorCommand::Ignore => {}
    }
}

fn insert_indent(editor: &mut crate::models::Editor, width: usize) {
    for _ in 0..width {
        editor.insert_char(' ');
    }
}

const fn editor_motion(motion: EditorNavigation) -> EditorMotion {
    match motion {
        EditorNavigation::Left => EditorMotion::Left,
        EditorNavigation::Right => EditorMotion::Right,
        EditorNavigation::Up => EditorMotion::Up(1),
        EditorNavigation::Down => EditorMotion::Down(1),
        EditorNavigation::Home => EditorMotion::Home,
        EditorNavigation::End => EditorMotion::End,
        EditorNavigation::PageUp => EditorMotion::Up(EDITOR_PAGE),
        EditorNavigation::PageDown => EditorMotion::Down(EDITOR_PAGE),
    }
}

fn save_editor(app: &mut App) {
    let Some(editor) = app.get_editor_mut() else {
        return;
    };
    let result = editor.save();
    let path = editor.path.clone();
    // The editor covers the panel's message row, so the status line alone
    // would report a refusal to a screen nobody can see. The bar is where the
    // user is already looking, and the next keystroke clears it.
    if let Err(ref err) = result {
        editor.notice = Some(format!("Cannot save: {err}"));
    }
    app.set_status(match result {
        Ok(()) => crate::status::Status::Saved(path),
        Err(err) => crate::status::Status::SaveFailed {
            error: err.to_string(),
        },
    });
    let _ = refresh_panels(app);
}

mod lsp;
#[cfg(test)]
mod tests;

fn close_editor(app: &mut App) {
    let Some(editor) = app.get_editor_mut() else {
        return;
    };
    if editor.state.is_modified() {
        editor.state = EditState::ConfirmingExit(ExitChoice::default());
    } else {
        app.close_overlay();
    }
}
fn process_encoding(app: &mut App, code: KeyCode) {
    use crate::encoding::TextEncoding;
    let Some(editor) = app.get_editor_mut() else {
        return;
    };
    let Some(choice) = editor.encoding_choice else {
        return;
    };
    let choices = TextEncoding::ALL;
    let index = choices
        .iter()
        .position(|encoding| *encoding == choice)
        .unwrap_or(0);
    match code {
        KeyCode::Esc => editor.encoding_choice = None,
        KeyCode::Up | KeyCode::Left => {
            editor.encoding_choice = Some(choices[(index + choices.len() - 1) % choices.len()]);
        }
        KeyCode::Down | KeyCode::Right | KeyCode::Tab => {
            editor.encoding_choice = Some(choices[(index + 1) % choices.len()]);
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
