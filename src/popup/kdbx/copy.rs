//! Purpose: copying a vault secret — Enter, Ctrl+U, and Ctrl+C in the edit form — to the clipboard with its auto-clear countdown, and Ctrl+R's password dialog.

use std::time::Instant;

use crate::clipboard::CopyRoute;
use crate::kdbx::Revealed;
use crate::models::App;
use crate::status::{Secret, Status};

/// `Ctrl+R`: open or close the selected entry's password dialog.
pub(super) fn toggle_reveal(app: &mut App, now: Instant) {
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
pub(super) fn copy(app: &mut App, what: Secret, now: Instant) {
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

pub(super) fn copy_edit_password(app: &mut App, now: Instant) {
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
