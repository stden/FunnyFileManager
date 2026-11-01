//! Purpose: the vault's own dialogs — the password dialog and the delete confirmation — and F8, Ctrl+Z and the notice they leave in the vault.

use std::time::Instant;

use crossterm::event::KeyCode;

use super::copy::copy;
use super::touch_session;
use crate::action::vault::VaultCommand;
use crate::kdbx::vault::Page;
use crate::models::App;
use crate::status::{Secret, Status};

/// Whether one of the vault's dialogs took the key, which the vault's keys
/// resolved to `command`.
pub(super) fn vault_dialog_input(
    app: &mut App,
    code: KeyCode,
    command: Option<VaultCommand>,
    now: Instant,
) -> bool {
    let page = app
        .kdbx_session()
        .and_then(crate::kdbx::KdbxSession::vault)
        .map(|vault| vault.page);
    let Some(page) = page else {
        return false;
    };
    match page {
        Page::Browse(_) => false,
        Page::Password(_) => password_dialog_input(app, code, command, now),
        Page::DeleteConfirm(_) => delete_dialog_input(app, code, now),
    }
}

/// Enter copies and Esc closes, as the dialog protocol has them; the key of
/// `Reveal`, which opened the dialog, closes it too.
fn password_dialog_input(
    app: &mut App,
    code: KeyCode,
    command: Option<VaultCommand>,
    now: Instant,
) -> bool {
    if code == KeyCode::Enter {
        copy(app, Secret::Password, now);
    } else if code == KeyCode::Esc || command == Some(VaultCommand::Reveal) {
        apply_command(app, VaultCommand::Reveal, now);
    }
    true
}

/// The password dialog's own typed command closes it and hides its revealed value.
pub(super) fn apply_command(app: &mut App, command: VaultCommand, now: Instant) -> bool {
    let password_dialog = app
        .kdbx_session()
        .and_then(crate::kdbx::KdbxSession::vault)
        .is_some_and(|vault| vault.form().is_none() && matches!(vault.page, Page::Password(_)));
    if command != VaultCommand::Reveal || !password_dialog {
        return false;
    }
    touch_session(app, |session| session.close_vault_dialog(now));
    true
}

fn delete_dialog_input(app: &mut App, code: KeyCode, now: Instant) -> bool {
    match code {
        KeyCode::Enter | KeyCode::Char('y' | 'Y') => confirm_delete(app, now),
        KeyCode::Esc | KeyCode::Char('n' | 'N') => {
            touch_session(app, |session| session.close_vault_dialog(now));
            app.set_status(Status::DeleteCancelled);
        }
        _ => {}
    }
    true
}

pub(super) fn request_delete(app: &mut App, now: Instant) {
    let result = app
        .kdbx_session_mut()
        .map_or(Err(crate::kdbx::KdbxError::Locked), |session| {
            session.request_delete(now)
        });
    if let Err(error) = result {
        app.set_status(Status::Error(error.to_string()));
    }
}

fn confirm_delete(app: &mut App, now: Instant) {
    let result = app
        .kdbx_session_mut()
        .map_or(Err(crate::kdbx::KdbxError::Locked), |session| {
            session.confirm_delete(now)
        });
    let status = match result {
        Ok(()) => Status::VaultEntryDeleted,
        Err(error) => Status::DeleteFailed {
            error: error.to_string(),
        },
    };
    set_vault_notice(app, &status);
    app.set_status(status);
}

pub(super) fn undo_vault_change(app: &mut App, now: Instant) {
    let result = app
        .kdbx_session_mut()
        .map_or(Err(crate::kdbx::KdbxError::Locked), |session| {
            session.undo_last_change(now)
        });
    let status = match result {
        Ok(()) => Status::VaultChangeUndone,
        Err(error) => Status::Error(error.to_string()),
    };
    set_vault_notice(app, &status);
    app.set_status(status);
}

fn set_vault_notice(app: &mut App, status: &Status) {
    if let Some(vault) = app
        .kdbx_session_mut()
        .and_then(crate::kdbx::KdbxSession::vault_mut)
    {
        vault.notice = Some(status.to_string());
    }
}
