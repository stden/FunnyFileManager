//! Purpose: the Attributes dialog's buttons — Enter is Set unless the cursor is on Cancel, and Space presses the button it is on.

use super::*;

/// Enter anywhere but on Cancel means Set — the default button — and Enter
/// on Cancel changes nothing.
#[cfg(unix)]
#[test]
fn enter_is_set_unless_the_cursor_is_on_cancel() {
    let temp = TempDir::new().expect("temp");
    let file = temp.path().join("a.txt");
    fs::write(&file, "x").expect("write");
    set_mode(&file, 0o644);

    let mut app = app_on(&temp, "a.txt");
    process_popup_input(&mut app, key(KeyCode::Char(' ')));
    process_popup_input(&mut app, key(KeyCode::BackTab));
    assert_eq!(dialog(&app).field, AttributesField::Cancel);
    process_popup_input(&mut app, key(KeyCode::Enter));
    assert!(!app.overlay.is_active());
    assert_eq!(mode(&file), 0o644, "Cancel wrote nothing");

    let mut app = app_on(&temp, "a.txt");
    process_popup_input(&mut app, key(KeyCode::Char(' ')));
    process_popup_input(&mut app, key(KeyCode::Enter));
    assert!(!app.overlay.is_active());
    assert_eq!(mode(&file), 0o244, "Enter from a box is Set");
}

/// Space on a button presses it, as Far's does: on Cancel the dialog closes
/// and nothing is written.
#[test]
fn space_on_cancel_presses_it() {
    let temp = TempDir::new().expect("temp");
    fs::write(temp.path().join("a.txt"), "x").expect("write");
    let mut app = app_on(&temp, "a.txt");
    process_popup_input(&mut app, key(KeyCode::BackTab));
    assert_eq!(dialog(&app).field, AttributesField::Cancel);
    process_popup_input(&mut app, key(KeyCode::Char(' ')));
    assert!(!app.overlay.is_active());
    assert!(
        !app.status_text().contains("Attributes set"),
        "{}",
        app.status_text()
    );
}

#[test]
fn space_on_set_presses_the_default_button() {
    let temp = TempDir::new().expect("temp");
    fs::write(temp.path().join("a.txt"), "x").expect("write");
    let mut app = app_on(&temp, "a.txt");
    while dialog(&app).field != AttributesField::Set {
        process_popup_input(&mut app, key(KeyCode::Tab));
    }
    process_popup_input(&mut app, key(KeyCode::Char(' ')));
    assert!(!app.overlay.is_active());
}
