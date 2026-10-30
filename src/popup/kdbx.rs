//! Purpose: keyboard handling for the `.kdbx` vault-browser overlay.
//!
//! Which key means what is the keymap's `vault` context, read by
//! `action::vault`; `browse` applies the command while the vault is browsed,
//! `form` takes `Save` and `Undo` from it in the entry form, and `dialogs`
//! keeps the dialog protocol of the password dialog and the delete question,
//! the password dialog closing on `Reveal` as well, the command that opened
//! it.
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

use std::time::Instant;

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::action::vault::{resolve, VaultCommand};
use crate::models::App;
use crate::status::Status;

mod browse;
mod copy;
mod dialogs;
mod form;

use browse::browse_command;
use copy::{copy, toggle_reveal};
use dialogs::{request_delete, undo_vault_change, vault_dialog_input};
use form::edit_input;

/// Handle one key for the vault browser. Always returns `true` — see the module
/// docs.
pub(super) fn process_kdbx_input(app: &mut App, code: KeyCode, modifiers: KeyModifiers) -> bool {
    let now = Instant::now();
    let key = KeyEvent::new(code, modifiers);
    let command = resolve(&app.keymap.vault, key);
    // Locking is answered first, in the form and the dialogs too.
    if command == Some(VaultCommand::Lock) {
        lock_and_close(app);
        return true;
    }
    if edit_input(app, key, command, now) {
        return true;
    }
    if vault_dialog_input(app, code, command, now) {
        return true;
    }
    if let Some(command) = command {
        browse_command(app, command, now);
    }
    true
}

/// Apply a command that arrived without a key — one of the unlock prompt's
/// buttons — as its key would be: the form and the vault's dialogs, which
/// read the key itself, are never up while the vault is locked.
pub fn apply(app: &mut App, command: VaultCommand) {
    browse_command(app, command, Instant::now());
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

/// `Ctrl+L`: overwrite every remembered key, clear any armed clipboard secret, and close the overlay.
fn lock_and_close(app: &mut App) {
    // Explicit rather than relying on the overlay being dropped: `lock()` is
    // what zeroizes the master key, and it must happen on the gesture the user
    // made, not whenever the enum field is next overwritten.
    touch_session(app, crate::kdbx::KdbxSession::lock);
    app.vault_cache = crate::kdbx::cache::SessionCache::default();
    if app.clipboard.is_armed() {
        app.clipboard.disarm();
        if let Some(backend) = app.clipboard.backend_mut() {
            drop(crate::clipboard::clear(backend));
        }
    }
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
