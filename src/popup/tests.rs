//! Unit tests and shared test helpers for `popup`.

use super::process_popup_input;

use crate::models::{App, Editor, Overlay};

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use std::path::PathBuf;

use tempfile::TempDir;

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

#[test]
fn test_editor_typing_inserts_and_marks_modified() {
    let mut app = app_with_editor("ab\n", PathBuf::from("test.txt"));

    assert!(process_popup_input(&mut app, key(KeyCode::Char('x'))));
    let editor = app.get_editor().expect("editor");
    assert_eq!(editor.lines[0], "xab");
    assert!(editor.state.is_modified());
}

#[test]
fn test_editor_enter_backspace_delete_and_navigation() {
    let mut app = app_with_editor("ab\n", PathBuf::from("test.txt"));

    process_popup_input(&mut app, key(KeyCode::Right));
    process_popup_input(&mut app, key(KeyCode::Enter));
    assert_eq!(app.get_editor().expect("editor").lines, vec!["a", "b"]);

    process_popup_input(&mut app, key(KeyCode::Backspace));
    assert_eq!(app.get_editor().expect("editor").lines, vec!["ab"]);

    process_popup_input(&mut app, key(KeyCode::Home));
    process_popup_input(&mut app, key(KeyCode::Delete));
    assert_eq!(app.get_editor().expect("editor").lines, vec!["b"]);

    process_popup_input(&mut app, key(KeyCode::End));
    assert_eq!(app.get_editor().expect("editor").cursor_col, 1);

    process_popup_input(&mut app, key(KeyCode::PageUp));
    assert_eq!(app.get_editor().expect("editor").cursor_line, 0);
    process_popup_input(&mut app, key(KeyCode::PageDown));
    process_popup_input(&mut app, key(KeyCode::Up));
    process_popup_input(&mut app, key(KeyCode::Down));
    process_popup_input(&mut app, key(KeyCode::Left));
    assert!(app.get_editor().is_some());
}

#[test]
fn test_editor_tab_inserts_four_spaces() {
    let mut app = app_with_editor("x\n", PathBuf::from("test.txt"));

    process_popup_input(&mut app, key(KeyCode::Tab));
    assert_eq!(app.get_editor().expect("editor").lines[0], "    x");
}

#[test]
fn test_editor_esc_unmodified_closes() {
    let mut app = app_with_editor("ab\n", PathBuf::from("test.txt"));

    assert!(process_popup_input(&mut app, key(KeyCode::Esc)));
    assert!(app.get_editor().is_none());
    assert!(!app.is_overlay_active());
}

#[test]
fn test_editor_esc_modified_asks_then_discards_on_n() {
    let temp_dir = TempDir::new().expect("temp dir");
    let path = temp_dir.path().join("file.txt");
    std::fs::write(&path, "ab\n").expect("write");
    let mut app = app_with_editor("ab\n", path.clone());

    process_popup_input(&mut app, key(KeyCode::Char('x')));
    process_popup_input(&mut app, key(KeyCode::Esc));
    assert!(app.get_editor().expect("editor").state.is_confirming_exit());

    // Esc in the confirm dialog returns to editing.
    process_popup_input(&mut app, key(KeyCode::Esc));
    assert!(!app.get_editor().expect("editor").state.is_confirming_exit());

    // Esc + N discards: overlay closed, file untouched.
    process_popup_input(&mut app, key(KeyCode::Esc));
    process_popup_input(&mut app, key(KeyCode::Char('n')));
    assert!(app.get_editor().is_none());
    assert_eq!(std::fs::read_to_string(&path).expect("read"), "ab\n");
    assert!(app.status_text().contains("discarded"));
}

#[test]
fn test_editor_esc_modified_saves_on_y() {
    let temp_dir = TempDir::new().expect("temp dir");
    let path = temp_dir.path().join("file.txt");
    std::fs::write(&path, "ab\n").expect("write");
    let mut app = app_with_editor("ab\n", path.clone());

    process_popup_input(&mut app, key(KeyCode::Char('x')));
    process_popup_input(&mut app, key(KeyCode::Esc));
    process_popup_input(&mut app, key(KeyCode::Char('y')));

    assert!(app.get_editor().is_none());
    assert_eq!(std::fs::read_to_string(&path).expect("read"), "xab\n");
    assert!(app.status_text().contains("Saved"));
}

/// Enter on the unsaved-changes dialog accepts whichever button already has
/// the focus — Save by default — without needing `Y`/`N`/`C` at all.
#[test]
fn test_editor_enter_on_exit_dialog_accepts_the_focused_choice() {
    let temp_dir = TempDir::new().expect("temp dir");
    let path = temp_dir.path().join("file.txt");
    std::fs::write(&path, "ab\n").expect("write");
    let mut app = app_with_editor("ab\n", path.clone());

    process_popup_input(&mut app, key(KeyCode::Char('x')));
    process_popup_input(&mut app, key(KeyCode::Esc));
    // Save is the default focus, so a bare Enter saves without ever typing y.
    process_popup_input(&mut app, key(KeyCode::Enter));

    assert!(app.get_editor().is_none());
    assert_eq!(std::fs::read_to_string(&path).expect("read"), "xab\n");
    assert!(app.status_text().contains("Saved"));
}

#[test]
fn test_editor_f2_saves_and_stays_open() {
    let temp_dir = TempDir::new().expect("temp dir");
    let path = temp_dir.path().join("file.txt");
    std::fs::write(&path, "ab\n").expect("write");
    let mut app = app_with_editor("ab\n", path.clone());

    process_popup_input(&mut app, key(KeyCode::Char('x')));
    process_popup_input(&mut app, key(KeyCode::F(2)));

    let editor = app.get_editor().expect("editor stays open");
    assert!(!editor.state.is_modified());
    assert_eq!(std::fs::read_to_string(&path).expect("read"), "xab\n");
    assert!(app.status_text().contains("Saved"));
}

#[test]
fn test_editor_save_failure_reports_error() {
    let temp_dir = TempDir::new().expect("temp dir");
    // Path inside a missing directory makes fs::write fail.
    let path = temp_dir.path().join("missing").join("file.txt");
    let mut app = app_with_editor("ab\n", path);

    process_popup_input(&mut app, key(KeyCode::Char('x')));
    process_popup_input(&mut app, key(KeyCode::F(2)));
    assert!(app.get_editor().expect("editor").state.is_modified());
    assert!(app.status_text().contains("Save failed"));

    // Save failure in the confirm dialog returns to editing.
    process_popup_input(&mut app, key(KeyCode::Esc));
    process_popup_input(&mut app, key(KeyCode::Char('y')));
    let editor = app.get_editor().expect("editor stays open");
    assert!(!editor.state.is_confirming_exit());
    assert!(app.status_text().contains("Save failed"));
}

#[test]
fn test_editor_ignores_unrelated_keys() {
    let mut app = app_with_editor("ab\n", PathBuf::from("test.txt"));

    assert!(process_popup_input(&mut app, key(KeyCode::F(5))));
    let editor = app.get_editor().expect("editor");
    assert!(!editor.state.is_modified());
    assert_eq!(editor.lines, vec!["ab"]);
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

/// Multi-byte whitespace (NBSP from a paste) used to panic tab-completion,
/// which aborts the process with the terminal still in raw mode.
#[test]
fn test_command_line_tab_completion_survives_multibyte_whitespace() {
    let temp = TempDir::new().expect("temp dir");
    std::fs::create_dir(temp.path().join("Documents")).expect("mkdir");

    let mut app = App::new().expect("test app");
    app.left_panel = crate::models::Panel::new(temp.path().to_path_buf()).expect("panel");
    app.active_panel = crate::models::PanelSide::Left;
    app.set_overlay(Overlay::CommandLine("cd\u{a0}Doc".into()));

    assert!(process_popup_input(&mut app, key(KeyCode::Tab)));
    assert!(matches!(app.overlay, Overlay::CommandLine(_)));
}

#[test]
fn refresh_helpers_bail_out_when_the_command_line_is_closed() {
    // Only the command-line overlay owns a menu/ghost; a stray call with any
    // other overlay must be a no-op instead of a panic.
    let mut app = App::new().expect("test app");
    super::command_line::refresh_menu(&mut app);
    super::command_line::refresh_ghost(&mut app);
    assert!(matches!(app.overlay, Overlay::None));
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

#[test]
fn tab_with_an_ambiguous_prefix_opens_the_candidate_menu() {
    let temp = TempDir::new().expect("temp dir");
    std::fs::create_dir(temp.path().join("alpha")).expect("dir");
    std::fs::create_dir(temp.path().join("alpine")).expect("dir");
    let mut app = App::new().expect("test app");
    app.left_panel = crate::models::Panel::new(temp.path().to_path_buf()).expect("panel");
    app.set_overlay(Overlay::CommandLine("cd al".into()));

    process_popup_input(&mut app, key(KeyCode::Tab));

    let Overlay::CommandLine(state) = &app.overlay else {
        panic!("command line must stay open");
    };
    assert!(
        state.menu.is_some(),
        "two candidates must open the menu, not silently pick one"
    );
}

#[test]
fn tab_with_a_single_candidate_completes_without_a_menu() {
    let temp = TempDir::new().expect("temp dir");
    std::fs::create_dir(temp.path().join("alpha")).expect("dir");
    let mut app = App::new().expect("test app");
    app.left_panel = crate::models::Panel::new(temp.path().to_path_buf()).expect("panel");
    app.set_overlay(Overlay::CommandLine("cd al".into()));

    process_popup_input(&mut app, key(KeyCode::Tab));

    let Overlay::CommandLine(state) = &app.overlay else {
        panic!("command line must stay open");
    };
    assert!(state.text().contains("alpha"), "{}", state.text());
    assert!(
        state.menu.is_none(),
        "a lone candidate is accepted outright, no menu"
    );
}

#[test]
fn completing_is_harmless_without_the_command_line() {
    // Tab only routes to the completer from the command-line overlay, but a
    // stray call must be a no-op rather than a panic.
    let mut app = App::new().expect("test app");
    super::command_line::complete_at_cursor(&mut app);
    assert!(matches!(app.overlay, Overlay::None));
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
