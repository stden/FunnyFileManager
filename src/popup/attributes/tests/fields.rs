//! Purpose: the Attributes dialog's other fields — typing into a time field, the subfolders checkbox, and a bad time keeping the dialog open.

use super::*;

/// Typing edits the time field under the cursor; Backspace takes it back.
#[test]
fn typing_edits_the_focused_time_field() {
    let temp = TempDir::new().expect("temp");
    fs::write(temp.path().join("a.txt"), "x").expect("write");
    let mut app = app_on(&temp, "a.txt");
    while dialog(&app).field != AttributesField::Accessed {
        process_popup_input(&mut app, key(KeyCode::Tab));
    }
    let before = dialog(&app).accessed.current.clone();
    process_popup_input(&mut app, key(KeyCode::Char('7')));
    assert_eq!(dialog(&app).accessed.current, format!("{before}7"));
    process_popup_input(&mut app, key(KeyCode::Backspace));
    assert_eq!(dialog(&app).accessed.current, before);

    // Ctrl+letter is a chord, not text.
    process_popup_input(
        &mut app,
        KeyEvent::new(KeyCode::Char('q'), KeyModifiers::CONTROL),
    );
    assert_eq!(dialog(&app).accessed.current, before);
    assert!(app.overlay.is_active());
}

/// Space on the subfolders box turns recursion on; a typed time that does not
/// parse leaves the dialog open with the reason in the status bar.
#[test]
fn subfolders_toggles_and_a_bad_time_keeps_the_dialog_open() {
    let temp = TempDir::new().expect("temp");
    fs::create_dir(temp.path().join("d")).expect("mkdir");
    let mut app = app_on(&temp, "d");
    while dialog(&app).field != AttributesField::Subfolders {
        process_popup_input(&mut app, key(KeyCode::Tab));
    }
    process_popup_input(&mut app, key(KeyCode::Char(' ')));
    assert!(dialog(&app).process_subfolders);

    process_popup_input(&mut app, key(KeyCode::BackTab));
    process_popup_input(&mut app, key(KeyCode::BackTab));
    assert_eq!(dialog(&app).field, AttributesField::Modified);
    for ch in "nope".chars() {
        process_popup_input(&mut app, key(KeyCode::Char(ch)));
    }
    process_popup_input(&mut app, key(KeyCode::Enter));
    assert!(app.overlay.is_active(), "the refusal is fixable in place");
    assert!(app.status_text().contains("nope"), "{}", app.status_text());
}
