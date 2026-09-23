//! Purpose: Cleanup keyboard behavior through the modal input dispatcher.

use std::fs;

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use tempfile::TempDir;

use crate::models::{App, CleanupField, Overlay};
use crate::test_support::disk_cleanup::{app, candidate};

fn press(app: &mut App, code: KeyCode) {
    assert!(crate::popup::process_popup_input(
        app,
        KeyEvent::new(code, KeyModifiers::NONE)
    ));
}

fn state(app: &App) -> &crate::models::DiskCleanupDialog {
    let Overlay::DiskCleanup(dialog) = &app.overlay else {
        panic!("cleanup should remain open");
    };
    dialog
}

#[test]
fn space_toggles_in_place_and_enter_advances_after_toggling() {
    let temp = TempDir::new().unwrap();
    let mut app = app(temp.path(), vec![candidate("Cache", temp.path())]);
    press(&mut app, KeyCode::Char(' '));
    assert!(!state(&app).candidates[0].checked);
    assert_eq!(state(&app).field, CleanupField::Item(0));
    press(&mut app, KeyCode::Enter);
    assert!(state(&app).candidates[0].checked);
    assert_eq!(state(&app).field, CleanupField::Run);
}

#[test]
fn arrows_and_tabs_cycle_controls_without_changing_selection() {
    let temp = TempDir::new().unwrap();
    let mut app = app(temp.path(), vec![candidate("Cache", temp.path())]);
    for (key, field) in [
        (KeyCode::Down, CleanupField::Run),
        (KeyCode::Tab, CleanupField::Cancel),
        (KeyCode::Tab, CleanupField::Item(0)),
        (KeyCode::Up, CleanupField::Cancel),
        (KeyCode::BackTab, CleanupField::Run),
        (KeyCode::Up, CleanupField::Item(0)),
    ] {
        press(&mut app, key);
        assert_eq!(state(&app).field, field);
        assert!(state(&app).candidates[0].checked);
    }
}

#[test]
fn missing_rows_and_buttons_do_not_toggle_and_unknown_keys_are_consumed() {
    let temp = TempDir::new().unwrap();
    let mut row = candidate("Missing", temp.path());
    row.exists = false;
    row.checked = false;
    let mut app = app(temp.path(), vec![row]);
    for key in [KeyCode::F(8), KeyCode::Char(' '), KeyCode::Enter] {
        press(&mut app, key);
        assert!(!state(&app).candidates[0].checked);
    }
    assert_eq!(state(&app).field, CleanupField::Run);
    press(&mut app, KeyCode::Char(' '));
    assert!(!state(&app).candidates[0].checked);
    assert_eq!(state(&app).field, CleanupField::Run);
}

#[test]
fn escape_f10_and_the_cancel_button_preserve_selected_files() {
    let temp = TempDir::new().unwrap();
    let file = temp.path().join("keep");
    fs::write(&file, b"keep").unwrap();
    for key in [KeyCode::Esc, KeyCode::F(10), KeyCode::Enter] {
        let mut app = app(temp.path(), vec![candidate("Cache", &file)]);
        press(&mut app, KeyCode::Up);
        press(&mut app, key);
        assert!(matches!(app.overlay, Overlay::None));
        assert_eq!(app.status_text(), "Cleanup cancelled");
        assert_eq!(fs::read(&file).unwrap(), b"keep");
    }
}

#[test]
fn enter_on_clean_up_removes_the_selected_file_and_reports_its_size() {
    let _guard = crate::utils::refresh_panels_mutex().lock().unwrap();
    let temp = TempDir::new().unwrap();
    let file = temp.path().join("cache");
    fs::write(&file, b"cache").unwrap();
    let mut app = app(temp.path(), vec![candidate("Cache", &file)]);
    press(&mut app, KeyCode::Down);
    press(&mut app, KeyCode::Enter);
    assert!(!file.exists());
    assert!(matches!(app.overlay, Overlay::None));
    assert_eq!(app.status_text(), "Freed 5 B");
}

#[test]
fn the_handler_declines_input_when_cleanup_is_closed() {
    let temp = TempDir::new().unwrap();
    let mut app = app(temp.path(), vec![]);
    app.close_overlay();
    assert!(!super::process(
        &mut app,
        KeyCode::Enter,
        KeyModifiers::NONE
    ));
}
