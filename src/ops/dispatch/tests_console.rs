//! Separate-console dispatch without starting an installed Codex session.

use super::*;
use crate::models::{Panel, PanelSide};
use tempfile::TempDir;

#[test]
fn codex_uses_the_active_panel_and_preserves_arguments() {
    let temp = TempDir::new().expect("tempdir");
    let mut app = tests::make_app(&temp);
    let right = temp.path().join("My Project");
    std::fs::create_dir(&right).expect("right directory");
    app.right_panel = Panel::new(right.clone()).expect("right panel");
    app.active_panel = PanelSide::Right;
    let input = r#"codex --model example "Explain this code""#;
    let mut launched = false;

    execute_command_with_console(&mut app, input, |command, directory| {
        assert_eq!(command.as_str(), input);
        assert_eq!(directory, right);
        launched = true;
        Ok(())
    })
    .expect("launch");

    assert!(launched);
    assert_eq!(
        app.status,
        Status::ConsoleStarted(ShellCommand::new(input).expect("command"))
    );
    let lines = app.console.visible(usize::MAX);
    assert_eq!(lines[0].text(), format!("{}> {input}", right.display()));
    assert_eq!(lines[1].text(), format!("Started in new console: {input}"));
    assert!(matches!(app.overlay, Overlay::None));
}

#[test]
fn console_launch_errors_reach_the_caller_and_transcript() {
    let temp = TempDir::new().expect("tempdir");
    let mut app = tests::make_app(&temp);
    let error = execute_command_with_console(&mut app, "codex", |_, _| {
        Err(io::Error::new(
            io::ErrorKind::PermissionDenied,
            "launch denied",
        ))
    })
    .expect_err("failed launch");

    assert!(error
        .to_string()
        .contains("cannot run 'codex': launch denied"));
    let lines = app.console.visible(usize::MAX);
    assert!(matches!(lines[1], crate::console::ConsoleLine::Error(_)));
    assert_eq!(lines[1].text(), error.to_string());
    assert!(!matches!(app.status, Status::ConsoleStarted(_)));
}

#[test]
fn ordinary_shell_commands_do_not_open_a_console() {
    let temp = TempDir::new().expect("tempdir");
    let mut app = tests::make_app(&temp);
    execute_command_with_console(&mut app, "echo codex", |_, _| {
        panic!("echo must stay in the captured shell")
    })
    .expect("echo");
    assert_eq!(app.status_text(), "codex");
}
