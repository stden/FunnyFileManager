//! Purpose: Verify directory and error propagation independently of platform command parsing.

use super::*;
use crate::models::{Panel, PanelSide};
use tempfile::TempDir;

fn app(root: &TempDir) -> App {
    App::with_panels(
        Panel::from_entries(root.path().join("left"), Vec::new()),
        Panel::from_entries(root.path().join("right"), Vec::new()),
    )
}

#[test]
fn console_effect_launches_in_the_active_directory_and_records_success() {
    let root = TempDir::new().unwrap();
    let mut app = app(&root);
    app.active_panel = PanelSide::Right;
    let command = ShellCommand::new("codex --help").unwrap();
    cmd_new_console(&mut app, &command, |received, directory| {
        assert_eq!(received.as_str(), command.as_str());
        assert_eq!(directory, root.path().join("right"));
        Ok(())
    })
    .unwrap();
    assert_eq!(app.status, Status::ConsoleStarted(command));
    assert_eq!(
        app.console.visible(1)[0].text(),
        "Started in new console: codex --help"
    );
}

#[test]
fn a_denied_console_launch_preserves_the_panel_and_records_an_error() {
    let root = TempDir::new().unwrap();
    let mut app = app(&root);
    let command = ShellCommand::new("codex").unwrap();
    let result = cmd_new_console(&mut app, &command, |_, _| {
        Err(io::ErrorKind::PermissionDenied.into())
    });
    assert!(result.is_err());
    assert!(!matches!(app.status, Status::ConsoleStarted(_)));
    assert!(matches!(
        app.console.visible(1)[0],
        crate::console::ConsoleLine::Error(_)
    ));
    assert_eq!(app.get_active_panel_ref().path, root.path().join("left"));
}
