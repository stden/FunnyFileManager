//! Purpose: the built-in editor's keys through the popup router — typing, navigation, Tab, Esc and its exit question, and F2.

use super::*;

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
