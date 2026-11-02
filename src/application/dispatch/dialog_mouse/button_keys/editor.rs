//! Purpose: the keys behind the built-in editor's dialog buttons — encoding, search, not found, exit and overwrite — told apart by the editor's own state.

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use super::first_or_esc;

/// The editor puts up three different dialogs, told apart by its own state.
pub(in crate::application::dispatch::dialog_mouse) fn dialog_button_key_in_editor(
    editor: &mut crate::models::Editor,
    index: usize,
) -> Option<KeyEvent> {
    use crate::models::EditorModal;
    let plain = |code| Some(KeyEvent::new(code, KeyModifiers::empty()));
    let char_key = |ch| plain(KeyCode::Char(ch));

    match editor.modal()? {
        EditorModal::Encoding(_) => plain(match index {
            0 => KeyCode::Enter,
            1 => KeyCode::Char('r'),
            _ => KeyCode::Esc,
        }),
        EditorModal::Search(_) => search_button_key(editor.search.prompt.as_mut()?, index),
        EditorModal::NotFound(_) => plain(KeyCode::Esc),
        EditorModal::ConfirmExit(_) => match index {
            0 => char_key('y'),
            1 => char_key('n'),
            _ => plain(KeyCode::Esc),
        },
        EditorModal::ConfirmOverwrite(_) => Some(first_or_esc(index, KeyCode::Char('y'))),
        EditorModal::Lsp => None,
    }
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
