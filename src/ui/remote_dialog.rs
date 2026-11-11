//! Purpose: Remote login masks credentials; operation dialogs use the standard controls.

use super::dialog::{self, Button, Ground, Row};
use crate::lineedit::LineEditor;
use crate::models::remote_dialog::{LoginField, RemoteDialog, RemoteLogin};
use crate::models::RemoteOverlay;
use crate::mouse::DialogMouse;
use ratatui::Frame;

type Caret = Option<(u16, u16)>;

pub(super) fn render(f: &mut Frame<'_>, tracker: &mut DialogMouse, overlay: &RemoteOverlay) {
    let caret = match &overlay.0 {
        RemoteDialog::Login(login) => render_login(f, tracker, login),
        RemoteDialog::Trust {
            login,
            fingerprint,
            answer,
        } => render_trust(f, tracker, login, fingerprint, answer.is_yes()),
        RemoteDialog::Input { .. } => render_input(f, tracker, &overlay.0),
        RemoteDialog::Delete { .. } => render_delete(f, tracker, &overlay.0),
    };
    if let Some(caret) = caret {
        f.set_cursor_position(caret);
    }
}

fn render_trust(
    f: &mut Frame<'_>,
    tracker: &mut DialogMouse,
    login: &RemoteLogin,
    fingerprint: &str,
    yes: bool,
) -> Caret {
    let host = login.session.url();
    dialog::render_tracked(
        f,
        tracker,
        "SSH host key",
        Ground::Prompt,
        &[
            Row::Plain(&host),
            Row::Plain(fingerprint),
            Row::Text("Trust this key for this connection?"),
        ],
        &[
            Button::new("&Yes").focused(yes),
            Button::new("&No").default_button().focused(!yes),
        ],
    )
}

fn render_input(f: &mut Frame<'_>, tracker: &mut DialogMouse, dialog: &RemoteDialog) -> Caret {
    let RemoteDialog::Input {
        purpose,
        input,
        cancel,
        notice,
        ..
    } = dialog
    else {
        return None;
    };
    dialog::render_tracked(
        f,
        tracker,
        purpose.title(),
        Ground::Prompt,
        &[
            Row::Label(purpose.label()),
            Row::line(input, !cancel, false),
            Row::Plain(notice),
        ],
        &[
            Button::new("&OK").default_button().focused(!cancel),
            Button::new("&Cancel").focused(*cancel),
        ],
    )
}

fn render_delete(f: &mut Frame<'_>, tracker: &mut DialogMouse, dialog: &RemoteDialog) -> Caret {
    let RemoteDialog::Delete {
        sources, answer, ..
    } = dialog
    else {
        return None;
    };
    let text = match sources.as_slice() {
        [only] => format!("Delete {}?", only.label()),
        many => format!("Delete {} selected objects?", many.len()),
    };
    dialog::render_tracked(
        f,
        tracker,
        "Delete on remote server",
        Ground::Prompt,
        &[
            Row::Text(&text),
            Row::Text("Directories and their contents will be removed."),
        ],
        &[
            Button::new("&Yes")
                .default_button()
                .focused(answer.is_yes()),
            Button::new("&No").focused(!answer.is_yes()),
        ],
    )
}

fn render_login(f: &mut Frame<'_>, tracker: &mut DialogMouse, login: &RemoteLogin) -> Caret {
    let host = login.session.url();
    let masked = LineEditor::new(&"*".repeat(login.credentials.password.chars().count()));
    let password = if matches!(login.session.auth, crate::models::Auth::KeyFile(_)) {
        "Key passphrase:"
    } else {
        "Password:"
    };
    let rows = [
        Row::Plain(&host),
        Row::Label("User:"),
        Row::line(&login.user, login.field == LoginField::User, false),
        Row::Label(password),
        Row::line(&masked, login.field == LoginField::Password, false),
        Row::Plain(&login.notice),
    ];
    dialog::render_tracked(
        f,
        tracker,
        "Connect to remote server",
        Ground::Prompt,
        &rows,
        &[
            Button::new("&Connect")
                .default_button()
                .focused(login.field == LoginField::Connect),
            Button::new("&Cancel").focused(login.field == LoginField::Cancel),
        ],
    )
}

#[cfg(test)]
mod tests;
