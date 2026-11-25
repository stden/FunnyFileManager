use super::key;
use crate::models::remote_dialog::{LoginField, RemoteLogin};
use crate::models::{ClipboardSlot, HostName, Port, Protocol, Session};
use crossterm::event::{KeyCode, KeyModifiers};

#[test]
fn password_paste_without_clipboard_backend_leaves_password_empty() {
    let session = Session::new(
        Protocol::Sftp,
        HostName::new("example.test").expect("host"),
        Port::SSH,
        None,
    );
    let mut login = RemoteLogin::new(session);
    login.field = LoginField::Password;
    let mut clipboard = ClipboardSlot::default();

    key(
        &mut login,
        KeyCode::Char('v'),
        KeyModifiers::CONTROL,
        &mut clipboard,
    );

    assert!(login.credentials.password.is_empty());
}
