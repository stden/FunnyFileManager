//! Purpose: button-to-key mappings for the remaining modal overlays.

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use super::plain;
use crate::models::{
    CleanupField, DiskCleanupDialog, FindFile, FindFileField, InputPrompt, Overlay, SessionEdit,
    SessionField,
};

pub(super) fn more_button_key(overlay: &mut Overlay, index: usize) -> Option<KeyEvent> {
    match overlay {
        Overlay::TempCleanup(dialog) => super::temp_cleanup::button(dialog, index),
        Overlay::Remote(dialog) => Some(super::remote_button_key(&mut dialog.0, index)),
        Overlay::FindFile(dialog) => Some(find_file_button_key(dialog, index)),
        Overlay::SessionEdit(dialog) => Some(session_edit_button_key(dialog, index)),
        Overlay::DiskCleanup(dialog) => Some(disk_cleanup_button_key(dialog, index)),
        Overlay::InputPrompt(prompt) => Some(input_prompt_button_key(prompt, index)),
        Overlay::Question(_) => Some(plain(KeyCode::Char(if index == 0 { 'y' } else { 'n' }))),
        Overlay::Things(_) => Some(plain(KeyCode::Esc)),
        Overlay::FindFolder(_)
        | Overlay::CommandHistory(_)
        | Overlay::ViewHistory(_)
        | Overlay::FolderHistory(_)
        | Overlay::UserMenu(_)
        | Overlay::Associations(_)
        | Overlay::FolderShortcuts(_)
        | Overlay::Filter(_)
        | Overlay::Screens(_)
        | Overlay::Hotplug(_)
        | Overlay::Settings(_) => Some(plain(KeyCode::Enter)),
        _ => None,
    }
}

fn find_file_button_key(dialog: &mut FindFile, index: usize) -> KeyEvent {
    let target = if index == 0 {
        FindFileField::Find
    } else {
        FindFileField::Cancel
    };
    while dialog.field() != target {
        dialog.tab(true);
    }
    plain(KeyCode::Enter)
}

fn session_edit_button_key(dialog: &mut SessionEdit, index: usize) -> KeyEvent {
    let target = if index == 0 {
        SessionField::Ok
    } else {
        SessionField::Cancel
    };
    while dialog.field() != target {
        dialog.tab(true);
    }
    plain(KeyCode::Enter)
}

fn disk_cleanup_button_key(dialog: &mut DiskCleanupDialog, index: usize) -> KeyEvent {
    dialog.field = if index == 0 {
        CleanupField::Run
    } else {
        CleanupField::Cancel
    };
    plain(KeyCode::Enter)
}

fn input_prompt_button_key(prompt: &mut InputPrompt, index: usize) -> KeyEvent {
    if prompt.on_cancel() != (index != 0) {
        prompt.toggle_button();
    }
    plain(KeyCode::Enter)
}

pub(super) fn archive_button_key(
    prompt: &mut crate::models::ArchivePrompt,
    index: usize,
) -> KeyEvent {
    prompt.field = if index == 0 {
        crate::models::ArchiveField::Add
    } else {
        crate::models::ArchiveField::Cancel
    };
    KeyEvent::new(KeyCode::Enter, KeyModifiers::empty())
}

pub(super) fn extract_button_key(
    prompt: &mut crate::models::ExtractPrompt,
    index: usize,
) -> KeyEvent {
    prompt.field = if index == 0 {
        crate::models::ExtractField::Extract
    } else {
        crate::models::ExtractField::Cancel
    };
    KeyEvent::new(KeyCode::Enter, KeyModifiers::empty())
}

/// Register / Cancel. A click on a dimmed Register does nothing: there is no
/// server to register.
pub(super) fn mcp_install_button_key(
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

pub(super) fn copy_button_key(
    prompt: &mut crate::copy::CopyPrompt,
    index: usize,
) -> Option<KeyEvent> {
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

/// "Retry as administrator" is `A`; OK, the only button or the second, Esc.
pub(super) fn failed_button(failure: &crate::models::OperationFailed, index: usize) -> KeyCode {
    if failure.retry().is_some() && index == 0 {
        KeyCode::Char('a')
    } else {
        KeyCode::Esc
    }
}
