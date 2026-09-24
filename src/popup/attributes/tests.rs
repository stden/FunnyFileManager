//! Unit tests: the keys of the File attributes dialog — the tab ring, Space
//! on a box, typing into a time field, and Enter on Set against Cancel.

use std::fs;
#[cfg(unix)]
use std::path::Path;

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use tempfile::TempDir;

use crate::models::{App, AttributesDialog, AttributesField, Overlay, Panel};
use crate::popup::process_popup_input;
use crate::popup::tests::key;

mod buttons;
mod fields;

fn app_on(temp: &TempDir, name: &str) -> App {
    let mut app = App::new().expect("test setup");
    app.left_panel = Panel::new(temp.path().to_path_buf()).expect("panel");
    app.left_panel.select_named(name);
    crate::ops::attributes::open(&mut app);
    assert!(matches!(app.overlay, Overlay::Attributes(_)));
    app
}

fn dialog(app: &App) -> &AttributesDialog {
    let Overlay::Attributes(dialog) = &app.overlay else {
        panic!("the attributes dialog is open");
    };
    dialog
}

#[cfg(unix)]
fn mode(path: &Path) -> u32 {
    use std::os::unix::fs::PermissionsExt;
    fs::metadata(path).expect("metadata").permissions().mode() & 0o777
}

#[cfg(unix)]
fn set_mode(path: &Path, mode: u32) {
    use std::os::unix::fs::PermissionsExt;
    fs::set_permissions(path, fs::Permissions::from_mode(mode)).expect("chmod");
}

/// Opening sets the variant; Esc takes it down and runs nothing else.
#[test]
fn open_sets_the_variant_and_esc_closes_it() {
    let temp = TempDir::new().expect("temp");
    fs::write(temp.path().join("a.txt"), "x").expect("write");
    let mut app = app_on(&temp, "a.txt");

    process_popup_input(&mut app, key(KeyCode::Char('x')));
    assert!(
        matches!(app.overlay, Overlay::Attributes(_)),
        "other keys are consumed"
    );

    process_popup_input(&mut app, key(KeyCode::Esc));
    assert!(!app.overlay.is_active());
}

/// Tab and the arrows walk the ring both ways.
#[test]
fn tab_and_arrows_move_the_cursor() {
    let temp = TempDir::new().expect("temp");
    fs::write(temp.path().join("a.txt"), "x").expect("write");
    let mut app = app_on(&temp, "a.txt");
    assert_eq!(dialog(&app).field, AttributesField::Flag(0));

    process_popup_input(&mut app, key(KeyCode::Tab));
    assert_eq!(dialog(&app).field, AttributesField::Flag(1));
    process_popup_input(&mut app, key(KeyCode::Down));
    assert_eq!(dialog(&app).field, AttributesField::Flag(2));
    process_popup_input(&mut app, key(KeyCode::Up));
    assert_eq!(dialog(&app).field, AttributesField::Flag(1));
    process_popup_input(&mut app, key(KeyCode::BackTab));
    process_popup_input(&mut app, key(KeyCode::BackTab));
    assert_eq!(
        dialog(&app).field,
        AttributesField::Cancel,
        "wraps backwards"
    );
}

/// Space flips the box under the cursor; Enter on Set writes it to disk.
#[cfg(unix)]
#[test]
fn space_flips_a_box_and_enter_on_set_writes_it() {
    let temp = TempDir::new().expect("temp");
    let file = temp.path().join("a.txt");
    fs::write(&file, "x").expect("write");
    set_mode(&file, 0o644);
    let mut app = app_on(&temp, "a.txt");

    // Flag(0) is owner read on Unix: [x] -> [ ].
    process_popup_input(&mut app, key(KeyCode::Char(' ')));
    assert_eq!(dialog(&app).flags[0].current, crate::models::Check::Clear);
    // Walk to Set: every box, the two times, then the button.
    while dialog(&app).field != AttributesField::Set {
        process_popup_input(&mut app, key(KeyCode::Tab));
    }
    process_popup_input(&mut app, key(KeyCode::Enter));

    assert!(!app.overlay.is_active());
    assert_eq!(mode(&file), 0o244);
    assert!(
        app.status_text().contains("1 object"),
        "{}",
        app.status_text()
    );
}

#[test]
fn f10_closes_and_side_arrows_step_along_the_flag_row() {
    let temp = TempDir::new().expect("temp");
    fs::write(temp.path().join("a.txt"), "x").expect("write");
    let mut app = app_on(&temp, "a.txt");
    process_popup_input(&mut app, key(KeyCode::Right));
    assert_eq!(dialog(&app).field, AttributesField::Flag(1));
    process_popup_input(&mut app, key(KeyCode::Left));
    assert_eq!(dialog(&app).field, AttributesField::Flag(0));
    process_popup_input(&mut app, key(KeyCode::F(10)));
    assert!(!app.overlay.is_active());
}

#[test]
fn helpers_do_nothing_when_the_dialog_is_not_open() {
    let mut app = App::new().expect("app");
    assert!(!super::process(
        &mut app,
        KeyCode::Enter,
        KeyModifiers::NONE
    ));
    super::press_space(&mut app);
    super::submit(&mut app);
    assert!(!app.overlay.is_active());
}
