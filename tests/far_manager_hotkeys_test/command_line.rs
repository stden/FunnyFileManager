//! Purpose: The command line: typing, editing, cancelling and running.

use super::*;

// ─── COMMAND LINE OVERLAY ────────────────────────────────────────────────────

#[test]
fn overlay_command_line_esc_cancels() {
    let temp = TempDir::new().unwrap();
    let mut app = setup(&temp).unwrap();
    app.set_overlay(Overlay::CommandLine("partial cmd".into()));
    process_popup_input(&mut app, press(KeyCode::Esc));
    assert!(app.get_command_line().is_none());
}

#[test]
fn overlay_command_line_char_appends() {
    let temp = TempDir::new().unwrap();
    let mut app = setup(&temp).unwrap();
    app.set_overlay(Overlay::CommandLine(String::new().into()));
    process_popup_input(&mut app, press(KeyCode::Char('p')));
    process_popup_input(&mut app, press(KeyCode::Char('w')));
    process_popup_input(&mut app, press(KeyCode::Char('d')));
    assert_eq!(app.get_command_line(), Some("pwd"));
}

#[test]
fn overlay_command_line_backspace_removes_char() {
    let temp = TempDir::new().unwrap();
    let mut app = setup(&temp).unwrap();
    app.set_overlay(Overlay::CommandLine("abc".into()));
    process_popup_input(&mut app, press(KeyCode::Backspace));
    assert_eq!(app.get_command_line(), Some("ab"));
}

#[test]
fn overlay_command_line_enter_executes_pwd() {
    let temp = TempDir::new().unwrap();
    let mut app = setup(&temp).unwrap();
    app.set_overlay(Overlay::CommandLine("pwd".into()));
    process_popup_input(&mut app, press(KeyCode::Enter));
    assert!(
        app.get_command_line().is_none(),
        "command line should close after Enter"
    );
    assert!(
        app.status_text().contains('/') || app.status_text().contains('\\'),
        "pwd should set status_message to a path"
    );
}

#[test]
fn overlay_command_line_enter_empty_is_noop() {
    let temp = TempDir::new().unwrap();
    let mut app = setup(&temp).unwrap();
    app.set_overlay(Overlay::CommandLine(String::new().into()));
    process_popup_input(&mut app, press(KeyCode::Enter));
    assert!(app.get_command_line().is_none());
}

/// A Ctrl+letter is a readline command, never text: Ctrl+C abandons the line
/// (bash's behaviour inside a prompt) and an unbound one leaves it alone. What
/// neither may ever do is append the bare letter.
#[test]
fn overlay_command_line_ctrl_char_is_never_typed() {
    let temp = TempDir::new().unwrap();
    let mut app = setup(&temp).unwrap();

    app.set_overlay(Overlay::CommandLine("ab".into()));
    process_popup_input(
        &mut app,
        press_mod(KeyCode::Char('c'), KeyModifiers::CONTROL),
    );
    assert_eq!(
        app.get_command_line(),
        Some(""),
        "Ctrl+C abandons the typed line"
    );

    app.set_overlay(Overlay::CommandLine("ab".into()));
    process_popup_input(
        &mut app,
        press_mod(KeyCode::Char('z'), KeyModifiers::CONTROL),
    );
    assert_eq!(
        app.get_command_line(),
        Some("ab"),
        "an unbound Ctrl+char must not append to the command line"
    );
}
