//! Purpose: the keystroke a dialog's button stands for.
//!
//! A click on a button is answered with the key that already does the job, so
//! the mouse and the keyboard cannot disagree. A dialog with a cursor of its
//! own is pointed at the clicked button first, so an old keyboard focus cannot
//! turn a click on OK into Cancel.

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::models::App;

/// The key a dialog's `index`-th button stands for.
///
/// A click is turned into the keystroke that already does the job, rather than
/// into a second path to the same effect. The mouse and the keyboard then
/// cannot disagree, and a dialog gains mouse support by naming its buttons
/// here instead of by growing a click handler of its own.
pub(super) fn dialog_button_key(app: &mut App, index: usize) -> Option<KeyEvent> {
    use crate::models::Overlay;

    match &mut app.overlay {
        // Yes / No.
        Overlay::QuitConfirm(_) => Some(plain(KeyCode::Char(if index == 0 { 'y' } else { 'n' }))),
        // Delete / Cancel, and Wipe / Cancel: the same two answers.
        Overlay::DeleteConfirm(_) | Overlay::WipeConfirm(_) => {
            Some(first_or_esc(index, KeyCode::Char('y')))
        }
        // OK / Cancel.
        Overlay::NewFileInput(_) => Some(first_or_esc(index, KeyCode::Enter)),
        // The same two, but the dialog has a cursor of its own: a click names
        // the button before Enter, so an old keyboard focus on Cancel cannot
        // make a click on OK cancel.
        Overlay::NamePrompt(prompt) => Some(name_button_key(prompt, index)),
        // OK / Cancel, with a cursor of its own like the name dialog's.
        Overlay::MaskPrompt(prompt) => {
            prompt.focus_cancel(index != 0);
            Some(plain(KeyCode::Enter))
        }
        // OK / Cancel with a cursor of its own, like the mask dialog's.
        Overlay::Describe(prompt) => Some(describe_button_key(prompt, index)),
        // Set / Cancel, with a cursor of its own like the name dialog's.
        Overlay::Attributes(dialog) => Some(attributes_button_key(dialog, index)),
        // OK / Cancel of the Apply command dialog, the same way.
        Overlay::ApplyCommand(prompt) => Some(apply_button_key(prompt, index)),
        // A single Ok.
        Overlay::Information(_) => Some(plain(KeyCode::Esc)),
        // Add / Cancel and Extract / Cancel, each with a cursor of its own
        // like the name dialog's.
        Overlay::ArchivePrompt(prompt) => Some(archive_button_key(prompt, index)),
        Overlay::ExtractPrompt(prompt) => Some(extract_button_key(prompt, index)),
        Overlay::McpInstall(dialog) => mcp_install_button_key(dialog, index),
        // Copy / Filter / Cancel. A click selects the named button before
        // Enter, so an old keyboard focus cannot make Copy act as Cancel.
        Overlay::CopyPrompt(prompt) => copy_button_key(prompt, index),
        Overlay::Editor(editor) => dialog_button_key_in_editor(editor, index),
        Overlay::MenuDialog(dialog) => menu_dialog_button_key(dialog, index),
        #[cfg(feature = "keepass")]
        Overlay::Kdbx(session) => kdbx_button_key(&session.0, index),
        _ => None,
    }
}

/// A key with no modifier.
fn plain(code: KeyCode) -> KeyEvent {
    KeyEvent::new(code, KeyModifiers::empty())
}

/// The first button sends `first`; every other button cancels.
fn first_or_esc(index: usize, first: KeyCode) -> KeyEvent {
    plain(if index == 0 { first } else { KeyCode::Esc })
}

#[cfg(feature = "keepass")]
pub(super) fn kdbx_button_key(
    session: &crate::kdbx::KdbxSession,
    index: usize,
) -> Option<KeyEvent> {
    if session.prompt().is_some() {
        return match index {
            0 => Some(KeyCode::Enter.into()),
            1 => Some(KeyEvent::new(KeyCode::Char('r'), KeyModifiers::CONTROL)),
            2 => Some(KeyCode::Esc.into()),
            _ => None,
        };
    }
    match session.vault()?.page {
        crate::kdbx::vault::Page::Password(_) | crate::kdbx::vault::Page::DeleteConfirm(_) => Some(
            (if index == 0 {
                KeyCode::Enter
            } else {
                KeyCode::Esc
            })
            .into(),
        ),
        crate::kdbx::vault::Page::Browse(_) => None,
    }
}

fn menu_dialog_button_key(dialog: &crate::models::MenuDialog, index: usize) -> Option<KeyEvent> {
    if !matches!(dialog, crate::models::MenuDialog::Prompt { .. }) {
        return None;
    }
    let code = match index {
        0 => KeyCode::Enter,
        1 => KeyCode::F(10),
        _ => return None,
    };
    Some(code.into())
}

fn attributes_button_key(dialog: &mut crate::models::AttributesDialog, index: usize) -> KeyEvent {
    dialog.focus_button(index);
    KeyCode::Enter.into()
}

fn apply_button_key(prompt: &mut crate::models::ApplyPrompt, index: usize) -> KeyEvent {
    prompt.focus_cancel(index != 0);
    KeyCode::Enter.into()
}

fn name_button_key(prompt: &mut crate::models::NamePrompt, index: usize) -> KeyEvent {
    prompt.focus(if index == 0 {
        crate::models::NameField::Ok
    } else {
        crate::models::NameField::Cancel
    });
    KeyCode::Enter.into()
}

fn describe_button_key(prompt: &mut crate::models::DescribePrompt, index: usize) -> KeyEvent {
    prompt.focus_cancel(index != 0);
    KeyCode::Enter.into()
}

fn archive_button_key(prompt: &mut crate::models::ArchivePrompt, index: usize) -> KeyEvent {
    prompt.field = if index == 0 {
        crate::models::ArchiveField::Add
    } else {
        crate::models::ArchiveField::Cancel
    };
    KeyEvent::new(KeyCode::Enter, KeyModifiers::empty())
}

fn extract_button_key(prompt: &mut crate::models::ExtractPrompt, index: usize) -> KeyEvent {
    prompt.field = if index == 0 {
        crate::models::ExtractField::Extract
    } else {
        crate::models::ExtractField::Cancel
    };
    KeyEvent::new(KeyCode::Enter, KeyModifiers::empty())
}

/// Register / Cancel. A click on a dimmed Register does nothing: there is no
/// server to register.
fn mcp_install_button_key(
    dialog: &mut crate::models::McpInstallDialog,
    index: usize,
) -> Option<KeyEvent> {
    dialog.field = match index {
        0 if dialog.can_register() => crate::models::McpInstallField::Register,
        0 => return None,
        _ => crate::models::McpInstallField::Cancel,
    };
    Some(plain(KeyCode::Enter))
}

fn copy_button_key(prompt: &mut crate::copy::CopyPrompt, index: usize) -> Option<KeyEvent> {
    let code = match index {
        0 => {
            prompt.field = crate::copy::CopyField::Copy;
            KeyCode::Enter
        }
        2 => KeyCode::Esc,
        _ => return None,
    };
    Some(KeyEvent::new(code, KeyModifiers::empty()))
}

/// The editor puts up three different dialogs, told apart by its own state.
pub(super) fn dialog_button_key_in_editor(
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
