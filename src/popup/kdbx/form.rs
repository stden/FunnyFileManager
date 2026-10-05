//! Purpose: keys of the vault's F4 edit form — typing, Tab, undo and F2 — and of its "save changes?" question.

use std::time::Instant;

use crossterm::event::{KeyCode, KeyModifiers};

use super::copy::copy_edit_password;
use crate::models::App;

pub(super) fn edit_input(
    app: &mut App,
    code: KeyCode,
    modifiers: KeyModifiers,
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
    if modifiers == KeyModifiers::CONTROL && matches!(code, KeyCode::Char('c' | 'C')) {
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
        confirm_edit_input(session, code, now);
    } else {
        edit_key(session, code, modifiers, now);
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
    code: KeyCode,
    modifiers: KeyModifiers,
    now: Instant,
) {
    let Some(form) = session.vault_mut().and_then(crate::kdbx::Vault::form_mut) else {
        return;
    };
    match code {
        KeyCode::F(2) => {
            session.save_edit(now);
        }
        KeyCode::Esc if form.is_dirty() => form.set_mode(crate::kdbx::FormMode::ConfirmSave),
        KeyCode::Esc => session.cancel_edit(now),
        KeyCode::BackTab => form.focus_prev(),
        KeyCode::Tab => form.focus_next(),
        KeyCode::Backspace => form.backspace(),
        KeyCode::Char('z' | 'Z') if modifiers == KeyModifiers::CONTROL => {
            form.undo();
        }
        KeyCode::Char(ch) if !modifiers.intersects(KeyModifiers::CONTROL | KeyModifiers::ALT) => {
            form.push_char(ch);
        }
        _ => {}
    }
}
