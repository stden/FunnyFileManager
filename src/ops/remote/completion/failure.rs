//! Purpose: Remote errors preserve edited text and return authentication to its own dialog.

use crate::models::remote_dialog::{LoginField, RemoteDialog, RemoteLogin};
use crate::models::remote_work::{RemoteRequest, RemoteTask};
use crate::models::{App, Confirm, EditState, Overlay, RemoteOverlay, Session};
use crate::remote::{Credentials, RemoteError};

pub(super) fn apply(app: &mut App, request: RemoteRequest, error: &RemoteError) {
    if save_error(app, &request.task, error) {
        return;
    }
    if !super::super::context::current(app, &request.target) {
        return;
    }
    if let RemoteTask::Connect {
        session,
        credentials,
    } = request.task
    {
        if !app.overlay.is_active() {
            connect_error(app, session, credentials, error);
        }
    }
    super::super::fail(app, &error.to_string());
}

fn save_error(app: &mut App, task: &RemoteTask, error: &RemoteError) -> bool {
    let RemoteTask::Save { source, then, .. } = task else {
        return false;
    };
    let Some(editor) = app.get_editor_mut().filter(|editor| {
        editor
            .remote
            .as_ref()
            .is_some_and(|remote| remote.location == *source)
    }) else {
        return true;
    };
    editor.state = if *error == RemoteError::Conflict {
        EditState::ConfirmingOverwrite(crate::models::OverwritePrompt::new(*then))
    } else {
        EditState::Modified
    };
    editor.notice = Some(format!("Not saved: {error}"));
    true
}

fn connect_error(app: &mut App, session: Session, credentials: Credentials, error: &RemoteError) {
    let mut login = RemoteLogin::new(session);
    login.user.set_text(&credentials.user);
    login.credentials = credentials;
    login.field = LoginField::Password;
    let dialog = if let RemoteError::HostKey(fingerprint) = error {
        RemoteDialog::Trust {
            login,
            fingerprint: fingerprint.clone(),
            answer: Confirm::No,
        }
    } else {
        login.notice = error.to_string();
        RemoteDialog::Login(login)
    };
    app.set_overlay(Overlay::Remote(Box::new(RemoteOverlay(dialog))));
}
