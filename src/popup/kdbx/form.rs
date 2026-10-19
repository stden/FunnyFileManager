//! Purpose: keys of the vault's F4 edit form — typing, Tab, the `Save` and `Undo` commands — and of its "save changes?" question.

use std::time::Instant;

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use super::copy::copy_edit_password;
use crate::action::vault::VaultCommand;
use crate::models::App;

/// Whether the entry form took `key`, resolved to `command` by the vault's
/// keys. The form's own keys are the dialog protocol; only `Save` and `Undo`
/// come from the command.
pub(super) fn edit_input(
    app: &mut App,
    key: KeyEvent,
    command: Option<VaultCommand>,
    now: Instant,
) -> bool {
    let editing = app
        .kdbx_session()
        .and_then(crate::kdbx::KdbxSession::vault)
        .and_then(crate::kdbx::Vault::form)
        .is_some();
    if !editing {
        return false;
    }
    // The form's Ctrl+C copies the password being edited; it is the form's
    // own key, not the vault's "lock and quit".
    if key.modifiers == KeyModifiers::CONTROL
        && matches!(key.code, KeyCode::Char('c' | 'C' | 'с' | 'С'))
    {
        copy_edit_password(app, now);
        return true;
    }
    let Some(session) = app.kdbx_session_mut() else {
        return false;
    };
    session.touch(now);
    let confirming = session
        .vault()
        .and_then(crate::kdbx::Vault::form)
        .is_some_and(|form| form.mode() == crate::kdbx::FormMode::ConfirmSave);
    if confirming {
        confirm_edit_input(session, key.code, now);
    } else {
        edit_key(session, key, command, now);
    }
    true
}

pub(super) fn confirm_edit_input(
    session: &mut crate::kdbx::KdbxSession,
    code: KeyCode,
    now: Instant,
) {
    match code {
        KeyCode::Enter | KeyCode::Char('y' | 'Y') => {
            session.save_edit(now);
        }
        KeyCode::Char('n' | 'N') => session.cancel_edit(now),
        KeyCode::Esc => {
            if let Some(form) = session.vault_mut().and_then(crate::kdbx::Vault::form_mut) {
                form.set_mode(crate::kdbx::FormMode::Editing);
            }
        }
        _ => {}
    }
}

fn edit_key(
    session: &mut crate::kdbx::KdbxSession,
    key: KeyEvent,
    command: Option<VaultCommand>,
    now: Instant,
) {
    let Some(form) = session.vault_mut().and_then(crate::kdbx::Vault::form_mut) else {
        return;
    };
    match (command, key.code) {
        (Some(VaultCommand::Save), _) => {
            session.save_edit(now);
        }
        (Some(VaultCommand::Undo), _) => {
            form.undo();
        }
        (_, KeyCode::Esc) if form.is_dirty() => form.set_mode(crate::kdbx::FormMode::ConfirmSave),
        (_, KeyCode::Esc) => session.cancel_edit(now),
        (_, KeyCode::BackTab) => form.focus_prev(),
        (_, KeyCode::Tab) => form.focus_next(),
        (_, KeyCode::Backspace) => form.backspace(),
        (_, KeyCode::Char(ch))
            if !key
                .modifiers
                .intersects(KeyModifiers::CONTROL | KeyModifiers::ALT) =>
        {
            form.push_char(ch);
        }
        _ => {}
    }
}
