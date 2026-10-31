//! Purpose: Remote dialogs own input and never expose passwords as ordinary text fields.

use crate::models::remote_dialog::RemoteDialog;
use crate::models::{App, Confirm, Overlay};
use crossterm::event::{KeyCode, KeyModifiers};

mod login;
mod submit;

enum Effect {
    Stay,
    Submit,
    Close,
}

pub(super) fn process(app: &mut App, code: KeyCode, modifiers: KeyModifiers) -> bool {
    let Overlay::Remote(dialog) = &mut app.overlay else {
        return false;
    };
    if code == KeyCode::Esc && !app.settings.enabled(crate::menu::SettingsPage::Dialog) {
        return true;
    }
    let effect = match code {
        KeyCode::Esc | KeyCode::F(10) => Effect::Close,
        KeyCode::Enter => Effect::Submit,
        _ => edit(&mut dialog.0, code, modifiers, &mut app.clipboard),
    };
    match effect {
        Effect::Stay => {}
        Effect::Submit => submit::run(app),
        Effect::Close => app.close_overlay(),
    }
    true
}

fn edit(
    dialog: &mut RemoteDialog,
    code: KeyCode,
    modifiers: KeyModifiers,
    clipboard: &mut crate::models::ClipboardSlot,
) -> Effect {
    match dialog {
        RemoteDialog::Login(login) => login::key(login, code, modifiers, clipboard),
        RemoteDialog::Trust { answer, .. } | RemoteDialog::Delete { answer, .. } => {
            return question_key(answer, code)
        }
        RemoteDialog::Input { input, cancel, .. } => match code {
            KeyCode::Tab | KeyCode::BackTab => *cancel = !*cancel,
            _ => {
                super::text_field::edit(input, code, modifiers, clipboard);
            }
        },
    }
    Effect::Stay
}

fn question_key(answer: &mut Confirm, code: KeyCode) -> Effect {
    match code {
        KeyCode::Tab | KeyCode::Left | KeyCode::Right => *answer = answer.toggled(),
        KeyCode::Char('y' | 'Y') => {
            *answer = Confirm::Yes;
            return Effect::Submit;
        }
        KeyCode::Char('n' | 'N') => return Effect::Close,
        _ => {}
    }
    Effect::Stay
}

pub(super) fn paste(app: &mut App, text: &str) {
    let Overlay::Remote(dialog) = &mut app.overlay else {
        return;
    };
    match &mut dialog.0 {
        RemoteDialog::Login(login) => login::paste(login, text),
        RemoteDialog::Input { input, .. } => input.insert_str(&super::text_field::one_line(text)),
        _ => {}
    }
}
