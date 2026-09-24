//! Purpose: the keys behind the built-in editor's dialog buttons — encoding, search, not found, exit and overwrite — told apart by the editor's own state.

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use super::first_or_esc;

/// The editor puts up three different dialogs, told apart by its own state.
pub(in crate::application::dispatch::dialog_mouse) fn dialog_button_key_in_editor(
    editor: &mut crate::models::Editor,
    index: usize,
) -> Option<KeyEvent> {
    let plain = |code| Some(KeyEvent::new(code, KeyModifiers::empty()));
    let char_key = |ch| plain(KeyCode::Char(ch));

    if editor.encoding_choice.is_some() {
        return plain(match index {
            0 => KeyCode::Enter,
            1 => KeyCode::Char('r'),
            _ => KeyCode::Esc,
        });
    }
    if let Some(prompt) = editor.search.prompt.as_mut() {
        return search_button_key(prompt, index);
    }
    if editor.search.not_found.is_some() {
        return plain(KeyCode::Esc);
    }
    if editor.state.exit_choice().is_some() {
        // Yes / No / Cancel.
        return match index {
            0 => char_key('y'),
            1 => char_key('n'),
            _ => plain(KeyCode::Esc),
        };
    }
    if editor.state.overwrite_prompt().is_some() {
        // Overwrite / Cancel.
        return Some(first_or_esc(index, KeyCode::Char('y')));
    }
    None
}

fn search_button_key(prompt: &mut crate::search::SearchPrompt, index: usize) -> Option<KeyEvent> {
    let plain = |code| Some(KeyEvent::new(code, KeyModifiers::empty()));
    match index {
        0 | 1 => {
            prompt.field = if index == 0 {
                crate::search::SearchField::FindPrevious
            } else {
                crate::search::SearchField::FindNext
            };
            plain(KeyCode::Enter)
        }
        _ => plain(KeyCode::Esc),
    }
}
