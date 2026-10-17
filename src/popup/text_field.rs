//! Purpose: The keys every one-line text field answers the same way: typing,
//! the caret, Shift-selection, select all, and copy, cut and paste through
//! the system clipboard.
//!
//! One place, so a field cannot forget Shift+Left or paste the way the rename
//! box did: every prompt built on a [`LineEditor`] hands its editing keys
//! here, and what each key means is the same in all of them — Windows' and
//! Far's own: Ctrl+C / Ctrl+Insert copy, Ctrl+X / Shift+Del cut, Ctrl+V /
//! Shift+Insert paste, Ctrl+A selects all.

use crossterm::event::{KeyCode, KeyModifiers};

use crate::lineedit::LineEditor;
use crate::models::{App, ClipboardSlot};

/// Answer `code` with `modifiers` in `input`; whether its text changed.
///
/// Keys a field has no use for (Tab, Enter, Esc, Ctrl chords it does not
/// know) are left alone, and the caller decides what they do.
pub fn edit(
    input: &mut LineEditor,
    code: KeyCode,
    modifiers: KeyModifiers,
    clipboard: &mut ClipboardSlot,
) -> bool {
    let before = input.text().to_owned();
    if !clipboard_key(input, code, modifiers, clipboard) {
        move_or_type(input, code, modifiers);
    }
    input.text() != before
}

/// Run `act` with the clipboard lent out of `app`, for a handler that has to
/// hold the whole `App` while it reaches its field (the task list's bars,
/// the LSP form). The slot goes back when `act` returns.
pub fn with_clipboard<T>(app: &mut App, act: impl FnOnce(&mut App, &mut ClipboardSlot) -> T) -> T {
    let mut clipboard = std::mem::take(&mut app.clipboard);
    let result = act(app, &mut clipboard);
    app.clipboard = clipboard;
    result
}

/// Copy, cut, paste and select all; whether `code` was one of them.
fn clipboard_key(
    input: &mut LineEditor,
    code: KeyCode,
    modifiers: KeyModifiers,
    clipboard: &mut ClipboardSlot,
) -> bool {
    let ctrl = modifiers.contains(KeyModifiers::CONTROL);
    let shift = modifiers.contains(KeyModifiers::SHIFT);
    match code {
        KeyCode::Char('a' | 'A') if ctrl => input.select_all(),
        KeyCode::Char('c' | 'C') | KeyCode::Insert if ctrl => {
            if let Some(text) = input.selected_text() {
                copy(clipboard, text);
            }
        }
        KeyCode::Char('x' | 'X') if ctrl => cut(input, clipboard),
        KeyCode::Delete if shift => cut(input, clipboard),
        KeyCode::Char('v' | 'V') if ctrl => paste(input, clipboard),
        KeyCode::Insert if shift => paste(input, clipboard),
        _ => return false,
    }
    true
}

/// Typing and the caret: Shift extends the selection, Ctrl moves by word.
fn move_or_type(input: &mut LineEditor, code: KeyCode, modifiers: KeyModifiers) {
    let word = modifiers.contains(KeyModifiers::CONTROL);
    let motion: fn(&mut LineEditor) = match code {
        KeyCode::Char(ch) if !modifiers.intersects(KeyModifiers::CONTROL | KeyModifiers::ALT) => {
            return input.insert_char(ch);
        }
        KeyCode::Backspace => return input.backspace(),
        KeyCode::Delete => return input.delete_forward(),
        KeyCode::Left if word => LineEditor::word_left,
        KeyCode::Right if word => LineEditor::word_right,
        KeyCode::Left => LineEditor::left,
        KeyCode::Right => LineEditor::right,
        KeyCode::Home => LineEditor::home,
        KeyCode::End => LineEditor::end,
        _ => return,
    };
    if modifiers.contains(KeyModifiers::SHIFT) {
        input.extend(motion);
    } else {
        motion(input);
    }
}

fn cut(input: &mut LineEditor, clipboard: &mut ClipboardSlot) {
    if let Some(text) = input.cut() {
        copy(clipboard, &text);
    }
}

fn paste(input: &mut LineEditor, clipboard: &mut ClipboardSlot) {
    let Some(text) = clipboard
        .backend_mut()
        .and_then(|backend| backend.get_native().ok())
    else {
        return;
    };
    input.insert_str(&one_line(&text));
}

/// Put `text` on the system clipboard. A copy that cannot be made is not
/// worth a message in a dialog: the text is still in the field.
pub fn copy(clipboard: &mut ClipboardSlot, text: &str) {
    if let Some(backend) = clipboard.backend_mut() {
        if crate::clipboard::copy_secret(backend, text).is_ok() {
            // Plain text: no pending clear of a copied password may wipe it.
            clipboard.disarm();
        }
    }
}

/// `text` as one line: a field has no second line to put the rest on, and a
/// name copied with its trailing newline must not end in one.
pub fn one_line(text: &str) -> String {
    text.trim_end_matches(['\r', '\n'])
        .split(['\r', '\n'])
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>()
        .join(" ")
}

#[cfg(test)]
mod tests;
