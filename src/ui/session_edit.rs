//! Purpose: Draws the session dialog (F4 / Shift+F4 on the sessions panel) —
//! every field of a stored session, laid out the way `NetBox` lays them out.
//!
//! Reads the dialog's state and nothing else; the keys that change it are
//! `popup::session_edit`'s. There is no password field: a password is asked
//! for when connecting and never stored.

use ratatui::Frame;

use super::dialog::{self, Button, Ground, Row};
use crate::models::{AuthKind, EditPurpose, Protocol, SessionEdit, SessionField};

const TITLE_NEW: &str = "New session";
const TITLE_EDIT: &str = "Edit session";
const PROTOCOLS: [&str; 4] = [
    Protocol::ALL[0].label(),
    Protocol::ALL[1].label(),
    Protocol::ALL[2].label(),
    Protocol::ALL[3].label(),
];
const AUTH: [&str; 3] = [
    AuthKind::ALL[0].label(),
    AuthKind::ALL[1].label(),
    AuthKind::ALL[2].label(),
];

pub(super) fn render(
    f: &mut Frame<'_>,
    tracker: &mut crate::mouse::DialogMouse,
    dialog: &SessionEdit,
) {
    let title = match dialog.purpose() {
        EditPurpose::Create => TITLE_NEW,
        EditPurpose::Edit(_) => TITLE_EDIT,
    };
    let buttons = [
        Button::new("&Ok")
            .default_button()
            .focused(dialog.field() == SessionField::Ok),
        Button::new("&Cancel").focused(dialog.field() == SessionField::Cancel),
    ];
    // The caret goes where typing lands: the end of the focused text field.
    if let Some(caret) =
        dialog::render_tracked(f, tracker, title, Ground::Prompt, &rows(dialog), &buttons)
    {
        f.set_cursor_position(caret);
    }
}

fn rows(d: &SessionEdit) -> Vec<Row<'_>> {
    let mut rows = login_rows(d);
    rows.extend(auth_rows(d));
    rows
}

/// A text field with its caption, focused when the cursor is on `field`.
fn text<'a>(d: &SessionEdit, caption: &'a str, value: &'a str, field: SessionField) -> Row<'a> {
    Row::Field {
        caption,
        value,
        focused: d.field() == field,
        enabled: d.is_enabled(field),
    }
}

fn login_rows(d: &SessionEdit) -> Vec<Row<'_>> {
    vec![
        text(d, "Name", &d.name, SessionField::Name),
        Row::Radio {
            caption: "Protocol",
            options: &PROTOCOLS,
            selected: position(&Protocol::ALL, &d.protocol),
            focused: d.field() == SessionField::Protocol,
        },
        text(d, "Host", &d.host, SessionField::Host),
        text(d, "Port", &d.port, SessionField::Port),
        text(d, "User", &d.user, SessionField::User),
    ]
}

fn auth_rows(d: &SessionEdit) -> Vec<Row<'_>> {
    vec![
        Row::Radio {
            caption: "Login",
            options: &AUTH,
            selected: position(&AuthKind::ALL, &d.auth),
            focused: d.field() == SessionField::Auth,
        },
        text(d, "Key file", &d.key_file, SessionField::KeyFile),
        text(d, "Directory", &d.remote_dir, SessionField::RemoteDir),
    ]
}

/// Where `chosen` is in `all`; the first choice for one that is not there.
fn position<T: PartialEq>(all: &[T], chosen: &T) -> usize {
    all.iter().position(|item| item == chosen).unwrap_or(0)
}
