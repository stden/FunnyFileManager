//! Purpose: keyboard handling for the `.kdbx` vault-browser overlay.
//!
//! Keys: `Enter` unlocks while locked and copies the password once unlocked,
//! `Ctrl+U` copies the user name, `Ctrl+R` opens the password dialog, printable characters
//! edit the password (locked) or the filter (unlocked), `Backspace` deletes,
//! `Up`/`Down`/`PgUp`/`PgDn` move, `Esc` remembers and closes, `Ctrl+L` locks.
//!
//! # Everything else is swallowed
//!
//! The `match` below has **no fall-through to the panel handler**, and the
//! function always returns `true`. This is not stylistic: a `_ => false` in the
//! command-line overlay once let keys through to the panel. Here `F8` deletes
//! only the selected `KeePass` entry after its own confirmation.
//!
//! # Secrets
//!
//! A copied value is wrapped in [`Revealed`] the moment it arrives, so it is
//! overwritten when this module is done with it rather than left in a freed
//! `String`. Nothing here logs, and every [`Status`] it produces is either
//! payload-free or carries a fixed label (see [`Secret`]).

use crate::models::Lifecycle;
use std::time::Instant;

use crossterm::event::{KeyCode, KeyModifiers};

use crate::clipboard::CopyRoute;
use crate::kdbx::{Motion, Revealed};
use crate::models::App;
use crate::status::{Secret, Status};

/// Rows `PgUp` / `PgDn` jump inside the entry list.
///
/// A fixed step, like the built-in editor's `EDITOR_PAGE`: the key handler does
/// not know the terminal size, and the overlay is 80% of the screen, so 10 rows
/// stays inside the visible window on the 24-row terminal the rest of the UI
/// assumes as its floor.
const VAULT_PAGE: usize = 10;

/// Handle one key for the vault browser. Always returns `true` — see the module
/// docs.
pub(super) fn process_kdbx_input(app: &mut App, code: KeyCode, modifiers: KeyModifiers) -> bool {
    let now = Instant::now();
    if modifiers == KeyModifiers::CONTROL && matches!(code, KeyCode::Char('l' | 'L')) {
        lock_and_close(app);
        return true;
    }
    if edit_input(app, code, modifiers, now) {
        return true;
    }
    if vault_dialog_input(app, code, modifiers, now) {
        return true;
    }
    if modifiers == KeyModifiers::CONTROL && matches!(code, KeyCode::Char('c' | 'C')) {
        lock_and_close(app);
        app.lifecycle = Lifecycle::Quitting;
        return true;
    }
    browse_key(app, code, modifiers, now);
    true
}

fn browse_key(app: &mut App, code: KeyCode, modifiers: KeyModifiers, now: Instant) {
    let ctrl = modifiers.contains(KeyModifiers::CONTROL);
    let alt = modifiers.contains(KeyModifiers::ALT);
    let locked = app
        .kdbx_session()
        .is_some_and(|session| session.state.is_locked());

    match code {
        KeyCode::Esc => remember_and_close(app),
        KeyCode::F(4) => begin_edit(app, now),
        KeyCode::F(6) => toggle_view(app),
        KeyCode::F(8) => request_delete(app, now),

        KeyCode::Char('r' | 'R') if ctrl && locked => toggle_master_password(app),
        KeyCode::Char('r' | 'R') if ctrl => toggle_reveal(app, now),
        KeyCode::Char('u' | 'U') if ctrl => copy(app, Secret::Username, now),
        KeyCode::Char('z' | 'Z') if ctrl => undo_vault_change(app, now),

        KeyCode::Enter if locked => unlock(app, now),
        KeyCode::Enter => copy(app, Secret::Password, now),

        KeyCode::Backspace => touch_session(app, |session| session.backspace(now)),
        KeyCode::Up => move_selection(app, Motion::Prev, now),
        KeyCode::Down => move_selection(app, Motion::Next, now),
        KeyCode::PageUp => move_selection(app, Motion::PageUp(VAULT_PAGE), now),
        KeyCode::PageDown => move_selection(app, Motion::PageDown(VAULT_PAGE), now),
        KeyCode::Home => move_selection(app, Motion::PageUp(usize::MAX), now),
        KeyCode::End => move_selection(app, Motion::PageDown(usize::MAX), now),

        // One entry point for printable input; `KdbxSession::type_char` routes it
        // to the masked password field or the filter, so a master password can
        // never be typed into the box that is rendered in clear.
        KeyCode::Char(ch) if !ctrl && !alt => touch_session(app, |session| {
            session.type_char(ch, now);
        }),

        // Deliberately empty: no key reaches the panels while this overlay is up.
        _ => {}
    }
}

fn toggle_view(app: &mut App) {
    touch_session(app, |session| {
        if let Some(vault) = session.vault_mut() {
            vault.toggle_structure();
        }
    });
}

fn remember_and_close(app: &mut App) {
    let overlay = std::mem::take(&mut app.overlay);
    if let crate::models::Overlay::Kdbx(session) = overlay {
        let unlocked = !session.0.state.is_locked();
        app.vault_cache.remember(session.0);
        app.set_status(if unlocked {
            Status::VaultRemembered
        } else {
            Status::VaultLocked
        });
    }
}

fn begin_edit(app: &mut App, now: Instant) {
    if let Some(session) = app.kdbx_session_mut() {
        if let Err(error) = session.begin_edit(now) {
            if let Some(vault) = session.vault_mut() {
                vault.notice = Some(error.to_string());
            }
        }
    }
}

fn vault_dialog_input(app: &mut App, code: KeyCode, modifiers: KeyModifiers, now: Instant) -> bool {
    use crate::kdbx::vault::Page;
    let page = app
        .kdbx_session()
        .and_then(crate::kdbx::KdbxSession::vault)
        .map(|vault| vault.page);
    let Some(page) = page else {
        return false;
    };
    match page {
        Page::Browse(_) => false,
        Page::Password(_) => password_dialog_input(app, code, modifiers, now),
        Page::DeleteConfirm(_) => delete_dialog_input(app, code, now),
    }
}

fn password_dialog_input(
    app: &mut App,
    code: KeyCode,
    modifiers: KeyModifiers,
    now: Instant,
) -> bool {
    if code == KeyCode::Enter {
        copy(app, Secret::Password, now);
    } else if code == KeyCode::Esc
        || (modifiers == KeyModifiers::CONTROL && matches!(code, KeyCode::Char('r' | 'R')))
    {
        touch_session(app, |session| session.close_vault_dialog(now));
    }
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

fn edit_input(app: &mut App, code: KeyCode, modifiers: KeyModifiers, now: Instant) -> bool {
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

fn confirm_edit_input(session: &mut crate::kdbx::KdbxSession, code: KeyCode, now: Instant) {
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

fn toggle_master_password(app: &mut App) {
    if let Some(prompt) = app
        .kdbx_session_mut()
        .and_then(crate::kdbx::KdbxSession::prompt_mut)
    {
        prompt.toggle_reveal();
    }
}

fn request_delete(app: &mut App, now: Instant) {
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

fn undo_vault_change(app: &mut App, now: Instant) {
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

/// Run `edit` against the open session, if there is one.
fn touch_session(app: &mut App, edit: impl FnOnce(&mut crate::kdbx::KdbxSession)) {
    if let Some(session) = app.kdbx_session_mut() {
        edit(session);
    }
}

fn move_selection(app: &mut App, motion: Motion, now: Instant) {
    touch_session(app, |session| session.move_selection(motion, now));
}

/// `Ctrl+L`: overwrite every remembered key and close the overlay.
fn lock_and_close(app: &mut App) {
    // Explicit rather than relying on the overlay being dropped: `lock()` is
    // what zeroizes the master key, and it must happen on the gesture the user
    // made, not whenever the enum field is next overwritten.
    touch_session(app, crate::kdbx::KdbxSession::lock);
    app.vault_cache = crate::kdbx::cache::SessionCache::default();
    app.close_overlay();
    app.set_status(Status::VaultLocked);
}

/// `Enter` while locked: try the typed credentials.
fn unlock(app: &mut App, now: Instant) {
    let Some(session) = app.kdbx_session_mut() else {
        return;
    };
    if !session.unlock(now) {
        // The reason stays in the prompt, beside the field it is about; the
        // status line only says that it failed. Nothing derived from the typed
        // password is allowed anywhere near `set_status`.
        app.set_status(Status::UnlockFailed);
        return;
    }
    let entries = session.vault().map_or(0, |vault| vault.entries().len());
    app.set_status(Status::VaultUnlocked { entries });
}

/// `Ctrl+R`: open or close the selected entry's password dialog.
fn toggle_reveal(app: &mut App, now: Instant) {
    let Some(session) = app.kdbx_session_mut() else {
        return;
    };
    // Only the failure is reported: announcing "revealed" in the status bar
    // would put the *fact* on screen twice and the value is already visible.
    if let Err(error) = session.reveal(now) {
        app.set_status(Status::Error(error.to_string()));
    }
}

/// `Enter` / `Ctrl+U`: put the selected entry's password / user name on the
/// clipboard and start the auto-clear countdown.
fn copy(app: &mut App, what: Secret, now: Instant) {
    let fetched = {
        let Some(session) = app.kdbx_session_mut() else {
            return;
        };
        match what {
            Secret::Password => session.copy_password(now),
            Secret::Username => {
                let username = session.copy_username();
                // `copy_username` takes `&self` and so cannot record the
                // activity itself; without this the idle lock would fire while
                // the user is actively copying user names.
                session.touch(now);
                username
            }
        }
    };

    let status = match fetched {
        Err(error) => Status::CopyFailed {
            error: error.to_string(),
        },
        Ok(secret) => {
            // Wrapped the instant it arrives: `Revealed` overwrites its bytes on
            // drop, so the plaintext is gone when this scope ends instead of
            // lingering in a freed `String` allocation until something reuses it.
            let secret = Revealed::new(secret);
            put_on_clipboard(app, secret.expose(), what, now)
        }
    };
    if let Some(vault) = app
        .kdbx_session_mut()
        .and_then(crate::kdbx::KdbxSession::vault_mut)
    {
        vault.notice = Some(status.to_string());
    }
    app.set_status(status);
}

fn copy_edit_password(app: &mut App, now: Instant) {
    let fetched = app
        .kdbx_session_mut()
        .map_or(Err(crate::kdbx::KdbxError::Locked), |session| {
            session.copy_edit_password(now)
        });
    let status = match fetched {
        Err(error) => Status::CopyFailed {
            error: error.to_string(),
        },
        Ok(secret) => {
            let secret = Revealed::new(secret);
            put_on_clipboard(app, secret.expose(), Secret::Password, now)
        }
    };
    app.set_status(status);
}

/// Copy `secret` through the installed backend and arm the auto-clear timer.
fn put_on_clipboard(app: &mut App, secret: &str, what: Secret, now: Instant) -> Status {
    let Some(backend) = app.clipboard.backend_mut() else {
        // Production installs a backend at startup (see `main_impl`), so this is
        // a test `App` or a session where no clipboard could be opened at all.
        return Status::CopyFailed {
            error: "no clipboard available".to_owned(),
        };
    };
    match crate::clipboard::copy_secret(backend, secret) {
        Ok(route) => {
            app.clipboard.arm(now);
            match route {
                CopyRoute::Native => Status::Copied(what),
                // Said out loud, because OSC 52 is silently dropped by terminals
                // that disable it: the write succeeds and nothing arrives.
                CopyRoute::Osc52 => Status::CopiedViaOsc52(what),
            }
        }
        // `copy_secret` redacts the value out of its own error text.
        Err(error) => Status::CopyFailed { error },
    }
}

#[cfg(test)]
mod tests;
