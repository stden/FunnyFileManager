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

use crate::kdbx::Motion;
use crate::models::App;
use crate::status::{Secret, Status};

mod copy;
mod dialogs;
mod form;

use copy::{copy, toggle_reveal};
use dialogs::{request_delete, undo_vault_change, vault_dialog_input};
use form::edit_input;

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

fn toggle_master_password(app: &mut App) {
    if let Some(prompt) = app
        .kdbx_session_mut()
        .and_then(crate::kdbx::KdbxSession::prompt_mut)
    {
        prompt.toggle_reveal();
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

#[cfg(test)]
mod tests;
