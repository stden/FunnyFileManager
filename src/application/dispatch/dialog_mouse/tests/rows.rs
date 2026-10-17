//! Purpose: clicks on a dialog's body rows: checkboxes answer, greyed rows and captions do not.

use super::*;

#[test]
fn clicking_a_copy_checkbox_row_ticks_it() {
    let dir = TempDir::new().expect("temp");
    let mut app = app_at(&dir);
    app.set_overlay(Overlay::CopyPrompt(Box::new(CopyPrompt::new(
        "a.txt",
        Path::new("/tmp"),
    ))));
    click_dialog_row(&mut app, 6);
    let Overlay::CopyPrompt(prompt) = &app.overlay else {
        panic!("copy dialog");
    };
    assert_eq!(prompt.field, CopyField::Timestamps);
    assert!(prompt.timestamps);
}

#[test]
fn clicking_a_greyed_copy_row_or_a_caption_does_nothing() {
    let dir = TempDir::new().expect("temp");
    let mut app = app_at(&dir);
    app.set_overlay(Overlay::CopyPrompt(Box::new(CopyPrompt::for_purpose(
        Purpose::Move,
        "a.txt",
        Path::new("/tmp"),
    ))));
    click_dialog_row(&mut app, 0);
    click_dialog_row(&mut app, 10);
    let Overlay::CopyPrompt(prompt) = &app.overlay else {
        panic!("copy dialog");
    };
    assert_eq!(prompt.field, CopyField::Destination);
    assert!(!prompt.filter);

    app.set_overlay(Overlay::Help);
    click_dialog_row(&mut app, 6);
    assert!(matches!(app.overlay, Overlay::Help));
}

// ─── clicking the rows of the Make folder dialog ────────────────────────────

/// A checkbox answers the click that reached it; every other control takes the
/// cursor and waits, which is what clicking a text field means everywhere.
#[test]
fn clicking_the_make_folder_rows_reaches_each_control() {
    use crate::models::NameField;

    let dir = TempDir::new().expect("temp");
    let mut app = app_at(&dir);
    app.set_overlay(Overlay::NamePrompt(NamePrompt::make_folder()));

    click_dialog_row(&mut app, 3);
    assert_eq!(field(&app), NameField::Link, "the drop-down");

    click_dialog_row(&mut app, 5);
    assert_eq!(field(&app), NameField::Multiple);
    let Overlay::NamePrompt(prompt) = &app.overlay else {
        panic!("still open");
    };
    assert!(prompt.multiple(), "the checkbox answered the click");

    click_dialog_row(&mut app, 1);
    assert_eq!(field(&app), NameField::Name);
}

/// A greyed control cannot be clicked into, and a row that is a caption or a
/// rule is not a control at all.
#[test]
fn clicking_a_grey_row_or_a_rule_changes_nothing() {
    use crate::models::NameField;

    let dir = TempDir::new().expect("temp");
    let mut app = app_at(&dir);
    app.set_overlay(Overlay::NamePrompt(NamePrompt::make_folder()));

    click_dialog_row(&mut app, 4);
    assert_eq!(field(&app), NameField::Name, "Target is greyed");
    click_dialog_row(&mut app, 0);
    assert_eq!(field(&app), NameField::Name, "row 0 is the caption");
    click_dialog_row(&mut app, 99);
    assert_eq!(field(&app), NameField::Name, "and there is no row 99");
}

/// A body click in a dialog that has no controls in its body must do nothing
/// rather than something the user cannot predict.
#[test]
fn clicking_the_body_of_a_dialog_without_controls_does_nothing() {
    let dir = TempDir::new().expect("temp");
    let mut app = app_at(&dir);
    app.set_overlay(Overlay::QuitConfirm(Confirm::Yes));
    click_dialog_row(&mut app, 0);
    assert!(matches!(app.overlay, Overlay::QuitConfirm(Confirm::Yes)));
}

fn field(app: &App) -> crate::models::NameField {
    let Overlay::NamePrompt(prompt) = &app.overlay else {
        panic!("the name dialog is open");
    };
    prompt.field()
}
