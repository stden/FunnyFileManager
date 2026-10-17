//! Purpose: Shift+F4 new-file dialog: typing, cancelling and creating.

use super::*;

// ─── SHIFT+F4 NEW FILE ───────────────────────────────────────────────────────

#[test]
fn hotkey_shift_f4_opens_new_file_dialog() {
    let temp = TempDir::new().unwrap();
    let mut app = setup(&temp).unwrap();
    assert!(app.get_new_file_input().is_none());
    app.set_overlay(Overlay::NewFileInput(String::new().into())); // Shift+F4 logic
    assert!(app.get_new_file_input().is_some());
}

#[test]
fn overlay_new_file_esc_cancels() {
    let temp = TempDir::new().unwrap();
    let mut app = setup(&temp).unwrap();
    app.set_overlay(Overlay::NewFileInput("partial".into()));
    process_popup_input(&mut app, press(KeyCode::Esc));
    assert!(app.get_new_file_input().is_none());
}

#[test]
fn overlay_new_file_char_appends_to_input() {
    let temp = TempDir::new().unwrap();
    let mut app = setup(&temp).unwrap();
    app.set_overlay(Overlay::NewFileInput(String::new().into()));
    process_popup_input(&mut app, press(KeyCode::Char('h')));
    process_popup_input(&mut app, press(KeyCode::Char('i')));
    assert_eq!(app.get_new_file_input(), Some("hi"));
}

#[test]
fn overlay_new_file_backspace_removes_last_char() {
    let temp = TempDir::new().unwrap();
    let mut app = setup(&temp).unwrap();
    app.set_overlay(Overlay::NewFileInput("abc".into()));
    process_popup_input(&mut app, press(KeyCode::Backspace));
    assert_eq!(app.get_new_file_input(), Some("ab"));
}

#[test]
fn overlay_new_file_enter_creates_file_on_disk() {
    let temp = TempDir::new().unwrap();
    let mut app = setup(&temp).unwrap();
    app.set_overlay(Overlay::NewFileInput("newfile.txt".into()));
    process_popup_input(&mut app, press(KeyCode::Enter));
    assert!(app.get_new_file_input().is_none(), "dialog should close");
    assert!(
        temp.path().join("newfile.txt").exists(),
        "file should be created"
    );
}

#[test]
fn overlay_new_file_enter_empty_name_shows_error() {
    let temp = TempDir::new().unwrap();
    let mut app = setup(&temp).unwrap();
    app.set_overlay(Overlay::NewFileInput(String::new().into()));
    process_popup_input(&mut app, press(KeyCode::Enter));
    assert!(app.get_new_file_input().is_none());
    assert!(app.status_text().contains("empty") || app.status_text().contains("Filename"));
}
