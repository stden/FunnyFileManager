//! Purpose: the terminal's own tools — Alt+F9's video mode and what it reports, and F12's screen list from the console.

use super::*;

/// Alt+F9 runs the same terminal resize as the F9 row. The resize itself is
/// skipped here so the test does not change the host terminal.
#[test]
fn video_mode_uses_the_terminal_toggle() {
    let _guard = crate::terminal::skip_video_mode_resize();
    let before = crate::terminal::video_mode_calls();
    let mut app = App::new().expect("test app");
    apply_action(
        &mut terminal(),
        &mut app,
        Action::Tool(ToolCommand::VideoMode),
    );
    assert!(!app.overlay.is_active());
    assert_eq!(crate::terminal::video_mode_calls(), before + 1);
    assert!(
        !app.status.to_string().contains("coming in this release"),
        "{}",
        app.status
    );
}

/// Alt+F9 must never be silent: the terminal may ignore the resize (tmux
/// does), so the status bar says what was asked for and what the terminal
/// has. The skipped resize stands for a terminal that kept its height.
#[test]
fn alt_f9_reports_what_the_terminal_did_with_the_resize() {
    let _guard = crate::terminal::skip_video_mode_resize();
    let mut app = App::new().expect("test app");
    let status_before = app.status.to_string();
    dispatch_key(
        &mut terminal(),
        &mut app,
        KeyEvent::new(KeyCode::F(9), KeyModifiers::ALT),
    );
    let status = app.status.to_string();
    assert_ne!(status, status_before, "Alt+F9 left the status unchanged");
    assert!(status.contains("Video mode"), "{status}");
}

/// F12 lists the screens from the console too, where every other panel
/// action is swallowed.
#[test]
fn f12_lists_the_screens_from_the_console() {
    let mut app = App::new().expect("test app");
    let mut terminal = terminal();
    app.view = ViewMode::Console;
    apply_action(&mut terminal, &mut app, Action::Tool(ToolCommand::Screens));
    assert!(matches!(app.overlay, Overlay::MenuDialog(_)));
    app.close_overlay();
    apply_action(&mut terminal, &mut app, Action::Tool(ToolCommand::FindFile));
    assert!(!app.overlay.is_active(), "other tools stay panel actions");
}
