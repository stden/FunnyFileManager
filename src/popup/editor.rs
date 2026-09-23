//! Editor input, with modal search and confirmation taking priority.

use super::{process_search_prompt, repeat_search, EDITOR_PAGE};
use crate::action::editor::{self as editor_keymap, EditorCommand, EditorNavigation};
use crate::models::{AfterSave, App, EditState, EditorMotion, ExitChoice};
use crate::search::SearchPrompt;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

/// A terminal paste is document text, never a sequence of editor shortcuts.
pub fn paste_text(app: &mut App, text: &str) {
    if let Some(editor) = app.get_editor_mut().filter(|editor| editor.accepts_text()) {
        editor.notice = None;
        editor.insert_text(text);
    }
}

pub(super) fn process_editor_input(app: &mut App, code: KeyCode, modifiers: KeyModifiers) -> bool {
    if app.get_editor().is_none() {
        return false;
    }
    if let Some(consumed) = process_editor_dialog(app, code, modifiers) {
        return consumed;
    }
    let Some(editor) = app.get_editor_mut() else {
        return false;
    };
    editor.notice = None;

    let command = editor_keymap::resolve(KeyEvent::new(code, modifiers));
    apply_editor_command(app, command);
    true
}

/// The key for whichever editor dialog is up, which owns the keyboard; `None`
/// when none is and the key belongs to the document.
fn process_editor_dialog(app: &mut App, code: KeyCode, modifiers: KeyModifiers) -> Option<bool> {
    let editor = app.get_editor_mut()?;
    if let Some(choice) = editor.state.exit_choice() {
        confirm_exit(app, code, choice);
        return Some(true);
    }
    if let Some(prompt) = editor.state.overwrite_prompt() {
        save::confirm_overwrite(app, code, prompt);
        return Some(true);
    }
    if editor.search.prompt.is_some() {
        return Some(process_search_prompt(app, code, modifiers));
    }
    if editor.search.not_found.is_some() {
        // The miss is drawn as Far's modal warning box, so the key that
        // dismisses it is spent on it: Enter must not also split the line,
        // nor Esc close the editor, nor a letter land in the text.
        editor.search.not_found = None;
        return Some(true);
    }
    if editor.encoding_choice.is_some() {
        process_encoding(app, code);
        return Some(true);
    }
    lsp::process(app, code).then_some(true)
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
        ExitChoice::Save => save::request_save(app, AfterSave::Close),
        ExitChoice::Discard => {
            app.close_overlay();
            app.set_status(crate::status::Status::ChangesDiscarded);
        }
        ExitChoice::Cancel => editor.state = EditState::Modified,
    }
}

/// Interpret the editor command language after modal input has been handled.
fn apply_editor_command(app: &mut App, command: EditorCommand) {
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
        EditorCommand::Save => save::save_editor(app),
        EditorCommand::Newline => editor.insert_newline(),
        EditorCommand::Backspace => editor.backspace(),
        EditorCommand::Delete => editor.delete_forward(),
        EditorCommand::Indent => insert_indent(editor),
        EditorCommand::Insert(ch) => editor.insert_char(ch),
        EditorCommand::Ignore => {}
    }
}

/// Tab: the file's own indentation — a TAB character in a file that uses
/// them, otherwise the tab size in spaces.
fn insert_indent(editor: &mut crate::models::Editor) {
    match editor.format.indent {
        crate::models::Indent::Tabs => editor.insert_char('\t'),
        crate::models::Indent::Spaces => {
            for _ in 0..editor.tab_size.columns() {
                editor.insert_char(' ');
            }
        }
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

mod lsp;
mod save;
#[cfg(test)]
mod tests;
#[cfg(test)]
mod tests_save;
#[cfg(test)]
mod tests_tab;

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
