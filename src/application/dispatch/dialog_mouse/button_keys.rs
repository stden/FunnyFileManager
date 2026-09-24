//! Purpose: the keystroke a dialog's button stands for.
//!
//! A click on a button is answered with the key that already does the job, so
//! the mouse and the keyboard cannot disagree. A dialog with a cursor of its
//! own is pointed at the clicked button first, so an old keyboard focus cannot
//! turn a click on OK into Cancel.

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::models::App;

mod editor;
#[cfg(feature = "keepass")]
mod vault;

pub(super) use editor::dialog_button_key_in_editor;
#[cfg(feature = "keepass")]
pub(super) use vault::kdbx_button_key;

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
        Overlay::LspEditor(dialog) => lsp_editor_button_key(dialog, index),
        // The task list's dialogs: each button its letter or its focus.
        Overlay::Tasks(list) => super::tasks::tasks_button_key(list, index),
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

/// The form's `{ OK }` / `[ Cancel ]`: the clicked button takes the cursor
/// before Enter, so an old keyboard focus cannot turn OK into Cancel. The exit
/// question's Yes / No / Cancel are its hotkeys. The list has no buttons.
fn lsp_editor_button_key(dialog: &mut crate::models::LspEditor, index: usize) -> Option<KeyEvent> {
    use crate::models::lsp_editor::{FormField, Mode};

    if matches!(dialog.mode(), Mode::ConfirmExit(_)) {
        return Some(match index {
            0 => plain(KeyCode::Char('y')),
            1 => plain(KeyCode::Char('n')),
            _ => plain(KeyCode::Esc),
        });
    }
    let form = dialog.form_mut()?;
    form.focus_on(if index == 0 {
        FormField::Ok
    } else {
        FormField::Cancel
    });
    Some(plain(KeyCode::Enter))
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
