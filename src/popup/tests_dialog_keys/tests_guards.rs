//! Purpose: the guards of the name, mask and folder dialogs — keys they have no use for, a taken or unchanged name, a blank one, and an entry that is gone.

use super::*;

// ─── the guards, and the keys the dialogs have no use for ───────────────────

/// A key neither dialog claims must leave it exactly as it was, rather than
/// falling through to whatever the panels would have done with it.
#[test]
fn a_key_the_name_or_mask_dialog_has_no_use_for_changes_nothing() {
    let (mut app, _dir) = app_over(&[]);
    app.set_overlay(Overlay::NamePrompt(NamePrompt::make_folder()));
    press(&mut app, KeyCode::Insert);
    assert_eq!(current_field(&app), NameField::Name);
    let Overlay::NamePrompt(prompt) = &app.overlay else {
        panic!("still open");
    };
    assert!(prompt.name().is_empty(), "and nothing was typed");

    app.set_overlay(Overlay::MaskPrompt(MaskPrompt::new(
        MaskPurpose::Select,
        "*",
    )));
    press(&mut app, KeyCode::Insert);
    let Overlay::MaskPrompt(prompt) = &app.overlay else {
        panic!("still open");
    };
    assert_eq!(prompt.mask(), "*");
}

/// Renaming onto a name that is taken refuses rather than overwriting: Far
/// asks, and this at least does not act.
#[test]
fn renaming_onto_a_name_that_is_taken_refuses() {
    use crate::models::FileName;

    let (mut app, dir) = app_over(&["a.txt", "b.txt"]);
    app.set_overlay(Overlay::NamePrompt(NamePrompt::rename(FileName::new(
        "a.txt",
    ))));
    for _ in 0..5 {
        press(&mut app, KeyCode::Backspace);
    }
    for ch in "b.txt".chars() {
        press(&mut app, KeyCode::Char(ch));
    }
    press(&mut app, KeyCode::Enter);

    assert!(
        dir.path().join("a.txt").exists(),
        "the original is still there"
    );
    assert_eq!(
        app.status,
        crate::status::Status::AlreadyExists("b.txt".into())
    );
}

/// F7 with several names makes each and counts them in the folder they went
/// into; the first name already taken stops the run and is the one named.
#[test]
fn make_folder_counts_what_it_made_and_names_what_was_taken() {
    use crate::status::Status;

    let (mut app, _dir) = app_over(&["taken"]);
    let directory = app.get_active_panel_ref().path.clone();
    let make = |app: &mut App, names: &str| {
        let mut prompt = NamePrompt::make_folder();
        prompt.focus(NameField::Multiple);
        prompt.toggle();
        prompt.focus(NameField::Name);
        names.chars().for_each(|ch| prompt.push(ch));
        app.set_overlay(Overlay::NamePrompt(prompt));
        press(app, KeyCode::Enter);
    };

    make(&mut app, "one;two");
    assert_eq!(
        app.status,
        Status::CreatedDirs {
            count: 2,
            directory
        }
    );
    make(&mut app, "three;taken");
    assert_eq!(app.status, Status::AlreadyExists("taken".into()));
}

/// Enter on a rename dialog whose name was never touched is a no-op: the
/// entry keeps its name, and nothing is even attempted.
#[test]
fn enter_on_an_unchanged_rename_does_nothing() {
    use crate::models::FileName;

    let (mut app, dir) = app_over(&["a.txt"]);
    app.set_overlay(Overlay::NamePrompt(NamePrompt::rename(FileName::new(
        "a.txt",
    ))));
    press(&mut app, KeyCode::Enter);

    assert!(!app.overlay.is_active(), "the dialog still closes");
    assert!(dir.path().join("a.txt").exists(), "nothing was renamed");
}

/// Enter on a "make folder" dialog whose name field was left blank does
/// nothing — a blank name is not a name to create.
#[test]
fn enter_on_a_blank_make_folder_name_does_nothing() {
    let (mut app, dir) = app_over(&[]);
    app.set_overlay(Overlay::NamePrompt(NamePrompt::make_folder()));
    press(&mut app, KeyCode::Enter);

    assert!(!app.overlay.is_active(), "the dialog still closes");
    assert_eq!(
        fs::read_dir(dir.path()).expect("read dir").count(),
        0,
        "no folder was created"
    );
}

/// A rename of something that is no longer on disk reports the failure rather
/// than claiming a rename that did not happen.
#[test]
fn renaming_something_that_is_gone_reports_it() {
    use crate::models::FileName;

    let (mut app, _dir) = app_over(&[]);
    app.set_overlay(Overlay::NamePrompt(NamePrompt::rename(FileName::new(
        "ghost.txt",
    ))));
    for _ in 0..9 {
        press(&mut app, KeyCode::Backspace);
    }
    for ch in "found.txt".chars() {
        press(&mut app, KeyCode::Char(ch));
    }
    press(&mut app, KeyCode::Enter);

    assert!(
        !app.status.to_string().starts_with("Renamed"),
        "{}",
        app.status
    );
}
