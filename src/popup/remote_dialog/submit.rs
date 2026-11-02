//! Purpose: Dialog submission queues one operation or keeps invalid input visible.

use crate::lineedit::LineEditor;
use crate::models::remote_dialog::{LoginField, RemoteDialog, RemoteInput};
use crate::models::remote_work::{RemoteTarget, RemoteTask};
use crate::models::{App, Overlay, RemoteOverlay};

pub(super) fn run(app: &mut App) {
    let Overlay::Remote(dialog) = std::mem::take(&mut app.overlay) else {
        return;
    };
    submit(app, dialog.0);
}

fn submit(app: &mut App, dialog: RemoteDialog) {
    match dialog {
        RemoteDialog::Login(login) if login.field != LoginField::Cancel => {
            crate::ops::remote::dialogs::login(app, login);
        }
        RemoteDialog::Trust {
            mut login,
            fingerprint,
            answer,
        } if answer.is_yes() => {
            login.credentials.trusted_host_key = Some(fingerprint);
            crate::ops::remote::dialogs::login(app, login);
        }
        RemoteDialog::Delete {
            target,
            sources,
            answer,
        } if answer.is_yes() => {
            crate::ops::remote::enqueue(app, target, RemoteTask::Delete(sources));
        }
        RemoteDialog::Input {
            target,
            purpose,
            input,
            cancel: false,
            ..
        } => input_answer(app, target, purpose, input),
        _ => {}
    }
}

fn input_answer(app: &mut App, target: RemoteTarget, purpose: RemoteInput, input: LineEditor) {
    if let Err(error) =
        crate::ops::remote::dialogs::input(app, target.clone(), purpose.clone(), input.text())
    {
        app.set_overlay(Overlay::Remote(Box::new(RemoteOverlay(
            RemoteDialog::Input {
                target,
                purpose,
                input,
                cancel: false,
                notice: error.to_string(),
            },
        ))));
    }
}
