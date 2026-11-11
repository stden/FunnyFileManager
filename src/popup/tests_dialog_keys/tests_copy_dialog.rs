//! Purpose: the F5 copy dialog's keys — walking its controls both ways, its checkboxes and drop-down, editing the destination, and cancelling.

use super::*;

// ─── the copy dialog ────────────────────────────────────────────────────────

fn open_copy(app: &mut App, destination: &std::path::Path) {
    app.set_overlay(Overlay::CopyPrompt(Box::new(CopyPrompt::for_purpose(
        Purpose::Copy,
        "a.txt",
        destination,
    ))));
}

fn copy_field(app: &App) -> CopyField {
    let Overlay::CopyPrompt(prompt) = &app.overlay else {
        panic!("the copy dialog is open");
    };
    prompt.field
}

#[test]
fn shift_tab_and_up_walk_the_copy_controls_backwards() {
    let (mut app, dir) = app_over(&["a.txt"]);
    open_copy(&mut app, dir.path());
    press(&mut app, KeyCode::BackTab);
    assert_eq!(
        copy_field(&app),
        CopyField::Cancel,
        "backwards from the top"
    );
    press(&mut app, KeyCode::Up);
    assert_eq!(copy_field(&app), CopyField::Copy);
}

/// Space toggles the checkbox under the cursor, and does nothing on a control
/// that is not one.
#[test]
fn space_toggles_a_checkbox_field_in_the_copy_dialog() {
    let (mut app, dir) = app_over(&["a.txt"]);
    open_copy(&mut app, dir.path());
    if let Overlay::CopyPrompt(prompt) = &mut app.overlay {
        prompt.field = CopyField::Timestamps;
    }
    press(&mut app, KeyCode::Char(' '));
    let Overlay::CopyPrompt(prompt) = &app.overlay else {
        panic!("still open");
    };
    assert!(prompt.timestamps, "space must toggle the checkbox");
}

/// Backspace and plain typing edit the destination field, and only that one —
/// Far uses the arrows to move the caret there instead of changing the answer.
#[test]
fn backspace_and_typing_edit_the_copy_destination() {
    let (mut app, dir) = app_over(&["a.txt"]);
    open_copy(&mut app, dir.path());
    let original = {
        let Overlay::CopyPrompt(prompt) = &app.overlay else {
            panic!("still open");
        };
        prompt.destination.text().to_owned()
    };

    press(&mut app, KeyCode::Backspace);
    press(&mut app, KeyCode::Char('x'));

    let Overlay::CopyPrompt(prompt) = &app.overlay else {
        panic!("still open");
    };
    let mut expected = original;
    expected.pop();
    expected.push('x');
    assert_eq!(prompt.destination.text(), expected);
}

#[test]
fn the_arrows_step_the_copy_dialogs_drop_down_both_ways() {
    let (mut app, dir) = app_over(&["a.txt"]);
    open_copy(&mut app, dir.path());
    press(&mut app, KeyCode::Tab);
    press(&mut app, KeyCode::Tab);
    assert_eq!(copy_field(&app), CopyField::Existing);

    press(&mut app, KeyCode::Right);
    let forward = existing(&app);
    press(&mut app, KeyCode::Left);
    assert_eq!(existing(&app), ExistingFiles::Ask, "and back again");
    assert_ne!(forward, ExistingFiles::Ask);
}

fn existing(app: &App) -> ExistingFiles {
    let Overlay::CopyPrompt(prompt) = &app.overlay else {
        panic!("the copy dialog is open");
    };
    prompt.existing
}

/// Enter on Cancel means cancel wherever else it would mean go, and an empty
/// destination is not something to act on — it would resolve to the working
/// directory, which is not what an emptied field means.
#[test]
fn the_copy_dialog_closes_without_copying_on_cancel_or_an_empty_destination() {
    let (mut app, dir) = app_over(&["a.txt"]);
    open_copy(&mut app, dir.path());
    if let Overlay::CopyPrompt(prompt) = &mut app.overlay {
        prompt.field = CopyField::Cancel;
    }
    press(&mut app, KeyCode::Enter);
    assert!(!app.overlay.is_active());

    open_copy(&mut app, dir.path());
    if let Overlay::CopyPrompt(prompt) = &mut app.overlay {
        prompt.destination.clear();
    }
    press(&mut app, KeyCode::Enter);
    assert!(!app.overlay.is_active(), "nothing to copy to");
}
