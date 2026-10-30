//! Purpose: the vault's browse commands applied — moving, copying, revealing, editing, locking and closing — while no form or dialog is up.

use std::time::Instant;

use super::{
    begin_edit, copy, lock_and_close, remember_and_close, request_delete, toggle_master_password,
    toggle_reveal, toggle_view, touch_session, undo_vault_change, unlock,
};
use crate::action::vault::VaultCommand;
use crate::kdbx::Motion;
use crate::models::{App, Lifecycle, ListMotion};
use crate::status::Secret;

/// Rows `PgUp` / `PgDn` jump inside the entry list.
///
/// A fixed step, like the built-in editor's `EDITOR_PAGE`: the key handler does
/// not know the terminal size, and the overlay is 80% of the screen, so 10 rows
/// stays inside the visible window on the 24-row terminal the rest of the UI
/// assumes as its floor.
const VAULT_PAGE: usize = 10;

/// One command on the browsed vault, locked or unlocked.
pub(super) fn browse_command(app: &mut App, command: VaultCommand, now: Instant) {
    let locked = app
        .kdbx_session()
        .is_some_and(|session| session.state.is_locked());
    match command {
        VaultCommand::Close => remember_and_close(app),
        VaultCommand::Edit => begin_edit(app, now),
        VaultCommand::ToggleView => toggle_view(app),
        VaultCommand::Delete => request_delete(app, now),
        VaultCommand::Reveal if locked => toggle_master_password(app),
        VaultCommand::Reveal => toggle_reveal(app, now),
        VaultCommand::CopyUsername => copy(app, Secret::Username, now),
        VaultCommand::Undo => undo_vault_change(app, now),
        VaultCommand::Activate if locked => unlock(app, now),
        VaultCommand::Activate => copy(app, Secret::Password, now),
        VaultCommand::Backspace => touch_session(app, |session| session.backspace(now)),
        VaultCommand::Move(motion) => move_selection(app, motion, now),
        // One entry point for printable input; `KdbxSession::type_char` routes
        // it to the masked password field or the filter, so a master password
        // can never be typed into the box that is rendered in clear.
        VaultCommand::Type(ch) => touch_session(app, |session| {
            session.type_char(ch, now);
        }),
        VaultCommand::Lock => lock_and_close(app),
        VaultCommand::LockAndQuit => {
            lock_and_close(app);
            app.lifecycle = Lifecycle::Quitting;
        }
        // The entry form's key: nothing to save while browsing.
        VaultCommand::Save => {}
    }
}

fn move_selection(app: &mut App, motion: ListMotion, now: Instant) {
    let motion = match motion {
        ListMotion::Up => Motion::Prev,
        ListMotion::Down => Motion::Next,
        ListMotion::PageUp => Motion::PageUp(VAULT_PAGE),
        ListMotion::PageDown => Motion::PageDown(VAULT_PAGE),
        ListMotion::Home => Motion::PageUp(usize::MAX),
        ListMotion::End => Motion::PageDown(usize::MAX),
    };
    touch_session(app, |session| session.move_selection(motion, now));
}
