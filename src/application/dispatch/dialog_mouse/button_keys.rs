//! Purpose: the keystroke a dialog's button stands for.
//!
//! A click on a button is answered with the key that already does the job, so
//! the mouse and the keyboard cannot disagree. A dialog with a cursor of its
//! own is pointed at the clicked button first, so an old keyboard focus cannot
//! turn a click on OK into Cancel.

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::models::App;

mod editor;
mod more;
#[cfg(feature = "keepass")]
mod vault;

pub(super) use editor::dialog_button_key_in_editor;
use more::{
    archive_button_key, copy_button_key, extract_button_key, failed_button, mcp_install_button_key,
    more_button_key,
};
#[cfg(feature = "keepass")]
pub(super) use vault::{kdbx_button_key, vault_prompt_button};

/// The key a dialog's `index`-th button stands for.
///
/// A click is turned into the keystroke that already does the job, rather than
/// into a second path to the same effect. The mouse and the keyboard then
/// cannot disagree, and a dialog gains mouse support by naming its buttons
/// here instead of by growing a click handler of its own.
pub(super) fn dialog_button_key(app: &mut App, index: usize) -> Option<KeyEvent> {
    use crate::models::{ExistingFile, Overlay};

    match &mut app.overlay {
        Overlay::QuitConfirm(_) => Some(plain(KeyCode::Char(if index == 0 { 'y' } else { 'n' }))),
        Overlay::DeleteConfirm(_) | Overlay::WipeConfirm(_) => {
            Some(first_or_esc(index, KeyCode::Char('y')))
        }
        Overlay::NewFileInput(_) => Some(first_or_esc(index, KeyCode::Enter)),
        Overlay::NamePrompt(prompt) => Some(name_button_key(prompt, index)),
        Overlay::MaskPrompt(prompt) => {
            prompt.focus_cancel(index != 0);
            Some(plain(KeyCode::Enter))
        }
        Overlay::Describe(prompt) => Some(describe_button_key(prompt, index)),
        Overlay::Attributes(dialog) => Some(attributes_button_key(dialog, index)),
        Overlay::ApplyCommand(prompt) => Some(apply_button_key(prompt, index)),
        Overlay::Information(_) => Some(plain(KeyCode::Esc)),
        Overlay::OperationFailed(failure) => Some(plain(failed_button(failure, index))),
        Overlay::ExistingFile(_) => Some(plain(KeyCode::Char(ExistingFile::button_hotkey(index)))),
        Overlay::ArchivePrompt(prompt) => Some(archive_button_key(prompt, index)),
        Overlay::ExtractPrompt(prompt) => Some(extract_button_key(prompt, index)),
        Overlay::McpInstall(dialog) => mcp_install_button_key(dialog, index),
        Overlay::CopyPrompt(prompt) => copy_button_key(prompt, index),
        Overlay::Editor(editor) => dialog_button_key_in_editor(editor, index),
        Overlay::MenuDialog(dialog) => menu_dialog_button_key(dialog, index),
        Overlay::LspEditor(dialog) => lsp_editor_button_key(dialog, index),
        Overlay::Tasks(list) => super::tasks::tasks_button_key(list, index),
        #[cfg(feature = "keepass")]
        Overlay::Kdbx(session) => kdbx_button_key(&session.0, index),
        other => more_button_key(other, index),
    }
}

/// A key with no modifier.
pub(super) fn plain(code: KeyCode) -> KeyEvent {
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
