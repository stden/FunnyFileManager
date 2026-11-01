//! Each remote operation should show the action and input expected by the user.
use super::*;
use crate::lineedit::LineEditor;
use crate::models::remote_dialog::{RemoteDialog, RemoteInput};
use crate::models::remote_work::{FileLocation, RemoteTarget};
use crate::models::{HostName, Port, Protocol, RemoteOverlay, Session};

fn assert_prompt(purpose: RemoteInput, title: &str, label: &str) {
    let temp = TempDir::new().unwrap();
    let mut app = app_at(&temp);
    let session = Session::new(
        Protocol::Sftp,
        HostName::new("example.test").unwrap(),
        Port::SSH,
        None,
    );
    let target = RemoteTarget::Connect {
        side: PanelSide::Left,
        session: session.name,
    };
    let dialog = RemoteDialog::Input {
        target,
        purpose,
        input: LineEditor::new("example"),
        cancel: false,
        notice: String::new(),
    };
    app.set_overlay(Overlay::Remote(Box::new(RemoteOverlay(dialog))));
    let mut term = terminal();
    draw(&mut app, &mut term);
    let screen = crate::test_support::buffer_string(term.backend().buffer());
    assert!(screen.contains(title), "{screen}");
    assert!(screen.contains(label), "{screen}");
}

#[test]
fn transfer_dialog_distinguishes_copy_and_move() {
    let location = FileLocation::Local("source".into());
    for (moving, title) in [(false, "Copy"), (true, "Move")] {
        let purpose = RemoteInput::Transfer {
            sources: vec![location.clone()],
            directory: location.clone(),
            moving,
        };
        assert_prompt(purpose, title, "Destination directory:");
    }
}

#[test]
fn rename_and_permissions_describe_the_expected_input() {
    let location = FileLocation::Local("source".into());
    assert_prompt(RemoteInput::Rename(location.clone()), "Rename", "New name:");
    assert_prompt(
        RemoteInput::Chmod(vec![location]),
        "Linux permissions",
        "Permissions (octal, for example 0644 or 0755):",
    );
}

#[test]
fn selection_dialog_distinguishes_select_and_deselect() {
    for (select, title) in [(true, "Select files"), (false, "Deselect files")] {
        assert_prompt(RemoteInput::Selection { select }, title, "File mask:");
    }
}
