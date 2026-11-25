//! Purpose: F1 help and closing the help overlay.

use super::*;

// ─── F1 HELP ────────────────────────────────────────────────────────────────

#[test]
fn hotkey_f1_toggles_help_on() {
    let temp = TempDir::new().unwrap();
    let mut app = setup(&temp).unwrap();
    assert!(!matches!(app.overlay, Overlay::Help));
    app.set_overlay(Overlay::Help); // F1 logic
    assert!(matches!(app.overlay, Overlay::Help));
}

#[test]
fn hotkey_f1_toggles_help_off() {
    let temp = TempDir::new().unwrap();
    let mut app = setup(&temp).unwrap();
    app.set_overlay(Overlay::Help);
    app.close_overlay();
    assert!(!matches!(app.overlay, Overlay::Help));
}

#[test]
fn overlay_help_esc_closes() {
    let temp = TempDir::new().unwrap();
    let mut app = setup(&temp).unwrap();
    app.set_overlay(Overlay::Help);
    process_popup_input(&mut app, press(KeyCode::Esc));
    assert!(!matches!(app.overlay, Overlay::Help));
}

#[test]
fn overlay_help_f1_closes() {
    let temp = TempDir::new().unwrap();
    let mut app = setup(&temp).unwrap();
    app.set_overlay(Overlay::Help);
    process_popup_input(&mut app, press(KeyCode::F(1)));
    assert!(!matches!(app.overlay, Overlay::Help));
}

#[test]
fn overlay_help_other_key_keeps_open() {
    let temp = TempDir::new().unwrap();
    let mut app = setup(&temp).unwrap();
    app.set_overlay(Overlay::Help);
    process_popup_input(&mut app, press(KeyCode::Down));
    assert!(
        matches!(app.overlay, Overlay::Help),
        "help should stay open on non-close keys"
    );
}
