//! Purpose: A remote password is masked at every draw and never enters command history.

use crate::models::remote_dialog::{LoginField, RemoteDialog, RemoteLogin};
use crate::models::{App, HostName, Overlay, Panel, Port, Protocol, RemoteOverlay, Session};
use ratatui::{backend::TestBackend, Terminal};

mod choices;
mod clipboard;
mod interaction;
mod operations;

fn display(dialog: RemoteDialog) -> String {
    let temp = tempfile::TempDir::new().expect("temporary directory");
    let panel = Panel::from_entries(temp.path().to_path_buf(), Vec::new());
    let mut app = App::with_panels(panel.clone(), panel);
    app.set_overlay(Overlay::Remote(Box::new(RemoteOverlay(dialog))));
    let mut terminal = Terminal::new(TestBackend::new(100, 24)).expect("terminal");
    terminal
        .draw(|frame| crate::ui::ui(frame, &mut app))
        .expect("draw");
    crate::test_support::buffer_string(terminal.backend().buffer())
}

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

#[test]
fn trust_dialog_shows_host_and_fingerprint() {
    let session = Session::new(
        Protocol::Sftp,
        HostName::new("example.test").expect("host"),
        Port::SSH,
        None,
    );
    let text = display(RemoteDialog::Trust {
        login: RemoteLogin::new(session),
        fingerprint: "SHA256:example".to_owned(),
        answer: crate::models::Confirm::No,
    });
    assert!(text.contains("SSH host key"));
    assert!(text.contains("example.test"));
    assert!(text.contains("SHA256:example"));
    assert!(text.contains("Trust this key"));
}

#[test]
fn input_dialog_shows_name_prompt_and_validation_message() {
    use crate::lineedit::LineEditor;
    use crate::models::remote_dialog::RemoteInput;
    use crate::models::remote_work::FileLocation;

    let text = display(RemoteDialog::Input {
        target: interaction::target(),
        purpose: RemoteInput::Mkdir(FileLocation::Local("directory".into())),
        input: LineEditor::new("new-folder"),
        cancel: false,
        notice: "Name is invalid".to_owned(),
    });
    assert!(text.contains("Make directory"));
    assert!(text.contains("Directory name:"));
    assert!(text.contains("new-folder"));
    assert!(text.contains("Name is invalid"));
}

#[test]
fn delete_dialog_describes_one_or_many_selected_items() {
    use crate::models::remote_work::FileLocation;

    for (sources, expected) in [
        (
            vec![FileLocation::Local("notes.txt".into())],
            "Delete notes.txt?",
        ),
        (
            vec![
                FileLocation::Local("one".into()),
                FileLocation::Local("two".into()),
            ],
            "Delete 2 selected objects?",
        ),
    ] {
        let text = display(RemoteDialog::Delete {
            target: interaction::target(),
            sources,
            answer: crate::models::Confirm::No,
        });
        assert!(text.contains(expected));
        assert!(text.contains("Directories and their contents will be removed."));
    }
}
