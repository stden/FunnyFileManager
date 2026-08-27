//! Purpose: keyboard handling for the `.kdbx` vault-browser overlay.
//!
//! Keys: `Enter` unlocks while locked and copies the password once unlocked,
//! `Ctrl+U` copies the user name, `Ctrl+R` toggles reveal, printable characters
//! edit the password (locked) or the filter (unlocked), `Backspace` deletes,
//! `Up`/`Down`/`PgUp`/`PgDn` move, `Esc` locks and closes.
//!
//! # Everything else is swallowed
//!
//! The `match` below has **no fall-through to the panel handler**, and the
//! function always returns `true`. This is not stylistic: a `_ => false` in the
//! command-line overlay once let `F8` through to the panel and delete files, and
//! the same bug in front of a password list would delete files the user cannot
//! even see. `kdbx_overlay_swallows_f8_without_touching_the_panel` pins it.
//!
//! # Secrets
//!
//! A copied value is wrapped in [`Revealed`] the moment it arrives, so it is
//! overwritten when this module is done with it rather than left in a freed
//! `String`. Nothing here logs, and every [`Status`] it produces is either
//! payload-free or carries a fixed label (see [`crate::status::Secret`]).

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
    let ctrl = modifiers.contains(KeyModifiers::CONTROL);
    let alt = modifiers.contains(KeyModifiers::ALT);
    let locked = app
        .kdbx_session()
        .is_some_and(|session| session.state.is_locked());

    match code {
        KeyCode::Esc => lock_and_close(app),

        // Ctrl+C keeps its global meaning (quit) instead of being repurposed as
        // "copy" — but it locks on the way out, so the master key is overwritten
        // at the gesture rather than only when `App` happens to be dropped.
        KeyCode::Char('c') if ctrl => {
            lock_and_close(app);
            app.lifecycle = Lifecycle::Quitting;
        }

        KeyCode::Char('r' | 'R') if ctrl => toggle_reveal(app, now),
        KeyCode::Char('u' | 'U') if ctrl => copy(app, Secret::Username, now),

        KeyCode::Enter if locked => unlock(app, now),
        KeyCode::Enter => copy(app, Secret::Password, now),

        KeyCode::Backspace => touch_session(app, |session| session.backspace(now)),
        KeyCode::Up => move_selection(app, Motion::Prev, now),
        KeyCode::Down => move_selection(app, Motion::Next, now),
        KeyCode::PageUp => move_selection(app, Motion::PageUp(VAULT_PAGE), now),
        KeyCode::PageDown => move_selection(app, Motion::PageDown(VAULT_PAGE), now),

        // One entry point for printable input; `KdbxSession::type_char` routes it
        // to the masked password field or the filter, so a master password can
        // never be typed into the box that is rendered in clear.
        KeyCode::Char(ch) if !ctrl && !alt => touch_session(app, |session| {
            session.type_char(ch, now);
        }),

        // Deliberately empty: no key reaches the panels while this overlay is up.
        _ => {}
    }
    true
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

/// `Esc`: overwrite the secrets, close the overlay, say so.
fn lock_and_close(app: &mut App) {
    // Explicit rather than relying on the overlay being dropped: `lock()` is
    // what zeroizes the master key, and it must happen on the gesture the user
    // made, not whenever the enum field is next overwritten.
    touch_session(app, crate::kdbx::KdbxSession::lock);
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

/// `Ctrl+R`: show or re-hide the selected entry's password.
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
