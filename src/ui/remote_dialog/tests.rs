//! Purpose: A remote password is masked at every draw and never enters command history.

use crate::models::remote_dialog::{LoginField, RemoteDialog, RemoteLogin};
use crate::models::{App, HostName, Overlay, Panel, Port, Protocol, RemoteOverlay, Session};
use ratatui::{backend::TestBackend, Terminal};

#[test]
fn login_keeps_passwords_out_of_the_screen_and_history() {
    let temp = tempfile::TempDir::new().expect("temporary directory");
    let panel = Panel::from_entries(temp.path().to_path_buf(), Vec::new());
    let mut app = App::with_panels(panel.clone(), panel);
    let session = Session::new(
        Protocol::Sftp,
        HostName::new("example.test").expect("host"),
        Port::SSH,
        None,
    );
    let mut login = RemoteLogin::new(session);
    login.credentials.password = "sample-secret".to_owned().into();
    login.field = LoginField::Password;
    app.set_overlay(Overlay::Remote(Box::new(RemoteOverlay(
        RemoteDialog::Login(login),
    ))));
    let mut terminal = Terminal::new(TestBackend::new(100, 24)).expect("terminal");
    terminal
        .draw(|frame| crate::ui::ui(frame, &mut app))
        .expect("draw");
    let text = crate::test_support::buffer_string(terminal.backend().buffer());
    assert!(text.contains("*************"));
    assert!(!text.contains("sample-secret"));
    assert!(text.contains("Connect to remote server"));
    assert_eq!(app.history.recall(crate::history::RingIndex::NEWEST), None);
}
