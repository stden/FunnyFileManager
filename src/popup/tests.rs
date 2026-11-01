//! Unit tests and shared test helpers for `popup`.

use super::process_popup_input;

use crate::models::{App, Editor, Overlay};

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use std::path::PathBuf;

use tempfile::TempDir;

mod completion;
mod editor_keys;

pub fn key(code: KeyCode) -> KeyEvent {
    KeyEvent::new(code, KeyModifiers::empty())
}

pub fn app_with_editor(content: &str, path: PathBuf) -> App {
    let mut app = App::new().expect("test app");
    app.set_overlay(Overlay::Editor(Editor::from_content(
        "test.txt".to_owned(),
        path,
        content,
    )));
    app
}

#[test]
fn command_line_consumes_f8_without_changing_overlay() {
    let mut app = App::new().expect("test app");
    app.set_overlay(Overlay::CommandLine("pwd".into()));

    let consumed = process_popup_input(
        &mut app,
        KeyEvent::new(KeyCode::F(8), KeyModifiers::empty()),
    );

    assert!(consumed);
    assert!(matches!(
        &app.overlay,
        Overlay::CommandLine(state) if state.text() == "pwd"
    ));
}

#[test]
fn command_line_consumes_alt_x_without_changing_overlay() {
    let mut app = App::new().expect("test app");
    app.set_overlay(Overlay::CommandLine("pwd".into()));

    let consumed = process_popup_input(
        &mut app,
        KeyEvent::new(KeyCode::Char('x'), KeyModifiers::ALT),
    );

    assert!(consumed);
    assert!(matches!(
        &app.overlay,
        Overlay::CommandLine(state) if state.text() == "pwd"
    ));
}

/// Ctrl+U is readline's "kill to start" inside the prompt — and, crucially,
/// still not the main view's "swap panels": the overlay consumes it either way.
#[test]
fn command_line_ctrl_u_kills_to_the_line_start_without_leaking() {
    let mut app = App::new().expect("test app");
    app.set_overlay(Overlay::CommandLine("pwd".into()));

    let consumed = process_popup_input(
        &mut app,
        KeyEvent::new(KeyCode::Char('u'), KeyModifiers::CONTROL),
    );

    assert!(consumed);
    assert!(matches!(
        &app.overlay,
        Overlay::CommandLine(state) if state.text().is_empty()
    ));
}

/// The `:` prompt is modal: keys it does not use must be swallowed, or the
/// event loop applies them to the panels behind the open prompt — Left/Right
/// move the panel whose path the command runs in, F5/F6/F8 operate on files,
/// F8 even replaces the typed text with a delete confirmation, and F10 quits.
///
/// The prompt now *uses* the cursor and kill keys, so they are exercised
/// separately below; what unites both lists is that neither reaches the panels.
#[test]
fn test_command_line_swallows_panel_keys() {
    let left = TempDir::new().expect("temp dir");
    let right = TempDir::new().expect("temp dir");

    let mut app = App::new().expect("test app");
    app.left_panel = crate::models::Panel::new(left.path().to_path_buf()).expect("panel");
    app.right_panel = crate::models::Panel::new(right.path().to_path_buf()).expect("panel");
    app.active_panel = crate::models::PanelSide::Right;
    app.set_overlay(Overlay::CommandLine("rm -rf build".into()));

    assert_stray_prompt_keys(&mut app);
    assert_owned_prompt_keys(&mut app);

    assert_eq!(app.active_panel, crate::models::PanelSide::Right);
    assert_eq!(app.get_active_panel().path, right.path());
    assert!(app.lifecycle.is_running());
}

#[test]
fn confirming_delete_of_a_vanished_file_counts_it_as_an_error() {
    let temp = TempDir::new().expect("temp dir");
    let mut app = App::new().expect("test app");
    let gone = temp.path().join("already-deleted.txt");
    app.set_overlay(Overlay::DeleteConfirm(vec![gone]));

    let consumed = process_popup_input(&mut app, key(KeyCode::Char('y')));

    assert!(consumed);
    // Per-file failures are folded into the summary status; the handler's
    // `Err` return (and popup's DeleteFailed arm) stays defensive-only.
    assert!(
        app.status_text().contains("Deleted 0 item(s), 1 error(s)"),
        "{}",
        app.status_text()
    );
}

fn assert_stray_prompt_keys(app: &mut App) {
    // Keys the prompt has no use for: consumed *and* leaving the text alone.
    let stray = [
        key(KeyCode::Insert),
        key(KeyCode::F(5)),
        key(KeyCode::F(6)),
        key(KeyCode::F(8)),
        key(KeyCode::F(10)),
        KeyEvent::new(KeyCode::Char('q'), KeyModifiers::ALT),
    ];

    for event in stray {
        assert!(
            process_popup_input(app, event),
            "{:?} leaked out of the command line",
            event.code
        );
        assert_eq!(
            app.get_command_line(),
            Some("rm -rf build"),
            "{:?} disturbed the typed command",
            event.code
        );
    }
}

fn assert_owned_prompt_keys(app: &mut App) {
    // Keys the prompt now owns: they edit the line rather than the panels, so
    // only the "did not reach the panels" half of the promise applies.
    let owned = [
        key(KeyCode::Left),
        key(KeyCode::Right),
        key(KeyCode::Up),
        key(KeyCode::Down),
        key(KeyCode::Home),
        key(KeyCode::End),
        KeyEvent::new(KeyCode::Char('u'), KeyModifiers::CONTROL),
    ];

    for event in owned {
        assert!(
            process_popup_input(app, event),
            "{:?} leaked out of the command line",
            event.code
        );
        assert!(
            matches!(app.overlay, Overlay::CommandLine(_)),
            "{:?} closed the prompt",
            event.code
        );
    }
}
