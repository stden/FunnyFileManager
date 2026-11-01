//! Editor input, with modal search and confirmation taking priority.

use super::{process_search_prompt, repeat_search, EDITOR_PAGE};
use crate::action::editor::{self as editor_keymap, EditorCommand, EditorNavigation};
use crate::models::{lsp::Intent, App, EditorMotion};
use crate::search::SearchPrompt;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use encoding::process_encoding;
use exit::{close_editor, confirm_exit};
pub(super) use exit::{exit_key, ExitKey};

/// The wheel over the text: a cursor move, as Up and Down are. `false` while
/// one of the editor's dialogs is up — it reads the key itself.
pub(super) fn wheel(app: &mut App, wheel: crate::mouse::Wheel, modifiers: KeyModifiers) -> bool {
    if !app
        .get_editor()
        .is_some_and(crate::models::Editor::accepts_text)
    {
        return false;
    }
    let motion = match wheel {
        crate::mouse::Wheel::Up => EditorNavigation::Up,
        crate::mouse::Wheel::Down => EditorNavigation::Down,
    };
    apply_editor_command(
        app,
        EditorCommand::Move {
            motion,
            selection: crate::action::editor::selection_mode(modifiers),
        },
    );
    true
}

/// A terminal paste is document text, or search-query text when F7 is open.
pub fn paste_text(app: &mut App, text: &str) {
    let Some(editor) = app.get_editor_mut() else {
        return;
    };
    match editor.modal() {
        None => {
            editor.notice = None;
            editor.insert_text(text);
        }
        Some(crate::models::EditorModal::Lsp)
            if matches!(
                editor.lsp.popup,
                Some(crate::models::lsp::Dialog::Completion { .. })
            ) =>
        {
            editor.notice = None;
            editor.lsp.completions = None;
            editor.insert_text(text);
        }
        Some(crate::models::EditorModal::Search(_)) => {
            if let Some(prompt) = editor
                .search
                .prompt
                .as_mut()
                .filter(|p| p.field == crate::search::SearchField::Query)
            {
                prompt.query.push_str(&super::text_field::one_line(text));
            }
        }
        _ => {}
    }
}

pub(super) fn process_editor_input(app: &mut App, code: KeyCode, modifiers: KeyModifiers) -> bool {
    if app.get_editor().is_none() {
        return false;
    }
    if let Some(consumed) = process_editor_dialog(app, code, modifiers) {
        return consumed;
    }
    let command = editor_keymap::resolve(&app.keymap.editor, KeyEvent::new(code, modifiers));
    apply_editor_command(app, command);
    true
}

/// The key for whichever editor dialog is up, which owns the keyboard; `None`
/// when none is and the key belongs to the document.
fn process_editor_dialog(app: &mut App, code: KeyCode, modifiers: KeyModifiers) -> Option<bool> {
    use crate::models::EditorModal;
    match app.get_editor()?.modal()? {
        EditorModal::Encoding(_) => {
            process_encoding(app, code);
            Some(true)
        }
        EditorModal::Search(_) => Some(process_search_prompt(app, code, modifiers)),
        EditorModal::NotFound(_) => {
            // The miss is drawn as Far's modal warning box, so the key that
            // dismisses it is spent on it: Enter must not also split the line,
            // nor Esc close the editor, nor a letter land in the text.
            if let Some(editor) = app.get_editor_mut() {
                editor.search.not_found = None;
            }
            Some(true)
        }
        EditorModal::ConfirmExit(choice) => {
            confirm_exit(app, code, choice);
            Some(true)
        }
        EditorModal::ConfirmOverwrite(prompt) => {
            save::confirm_overwrite(app, code, prompt);
            Some(true)
        }
        EditorModal::Lsp => lsp::process_key(app, code, modifiers),
    }
}

/// Interpret the editor command language after modal input has been handled.
pub fn apply_editor_command(app: &mut App, command: EditorCommand) {
    let Some(editor) = app.get_editor_mut().filter(|editor| editor.accepts_text()) else {
        return;
    };
    editor.notice = None;
    match command {
        EditorCommand::Move { motion, selection } => {
            editor.move_cursor(editor_motion(motion), selection);
        }
        EditorCommand::Copy => crate::ops::copy_editor_selection(app),
        EditorCommand::InsertFileName => crate::ops::insert_editor_file_name(app),
        EditorCommand::SelectAll => editor.select_all(),
        EditorCommand::ChooseEncoding => editor.encoding_choice = Some(editor.format.encoding),
        EditorCommand::ToggleCodePage => crate::ops::toggle_editor_code_page(app),
        EditorCommand::Paste => crate::ops::paste_into_editor(app),
        EditorCommand::RepeatSearch => repeat_search(app),
        EditorCommand::FormatDocument => editor.format_document(),
        EditorCommand::Lsp(feature) => editor.lsp.intent = Intent::Query(feature),
        EditorCommand::RestartLsp => editor.lsp.intent = Intent::Restart,
        EditorCommand::Search => {
            editor.search.prompt = Some(SearchPrompt::from_last(editor.search.last.as_ref()));
        }
        EditorCommand::Close => close_editor(app),
        EditorCommand::Save => save::save_editor(app),
        EditorCommand::Newline => editor.insert_newline(),
        EditorCommand::Backspace => editor.backspace(),
        EditorCommand::Delete => editor.delete_forward(),
        EditorCommand::Indent => insert_indent(editor),
        EditorCommand::Insert(ch) => insert_char_and_complete(editor, ch),
        EditorCommand::Ignore => {}
    }
}

fn insert_char_and_complete(editor: &mut crate::models::Editor, ch: char) {
    editor.insert_char(ch);
    editor.maybe_trigger_completion(ch);
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
        EditorNavigation::DocumentStart => EditorMotion::DocumentStart,
        EditorNavigation::DocumentEnd => EditorMotion::DocumentEnd,
    }
}

mod encoding;
mod exit;
mod lsp;
mod save;
#[cfg(test)]
mod tests;
#[cfg(test)]
mod tests_completion_paste;
#[cfg(test)]
mod tests_save;
#[cfg(test)]
mod tests_tab;
#[cfg(test)]
mod tests_without_editor;
