//! Purpose: the button index → key map of every plain dialog, via `dialog_button_key`.

use super::*;

/// The describe dialog has a cursor of its own, so a click names the button
/// before Enter: OK writes even when the keyboard focus was on Cancel.
#[test]
fn describe_buttons_move_the_cursor_and_press_enter() {
    let dir = TempDir::new().expect("temp");
    let mut app = app_at(&dir);
    let mut prompt = crate::models::DescribePrompt::new("a.txt".into(), "", Vec::new());
    prompt.toggle_button();
    app.set_overlay(Overlay::Describe(prompt));
    assert_eq!(code(&mut app, 0), KeyCode::Enter);
    let Overlay::Describe(prompt) = &app.overlay else {
        panic!("describe")
    };
    assert!(!prompt.on_cancel(), "the click lands on OK");
    assert_eq!(code(&mut app, 1), KeyCode::Enter);
    let Overlay::Describe(prompt) = &app.overlay else {
        panic!("describe")
    };
    assert!(prompt.on_cancel(), "the click lands on Cancel");
}

/// A click on Set or Cancel moves the cursor there before Enter, so an old
/// keyboard focus on Cancel cannot make a click on Set cancel.
#[test]
fn attributes_buttons_name_set_and_cancel_before_enter() {
    use crate::models::{AttributesDialog, AttributesField};
    let dir = TempDir::new().expect("temp");
    let mut app = app_at(&dir);
    app.set_overlay(Overlay::Attributes(Box::<AttributesDialog>::default()));
    for (index, field) in [(1, AttributesField::Cancel), (0, AttributesField::Set)] {
        assert_eq!(code(&mut app, index), KeyCode::Enter);
        let Overlay::Attributes(dialog) = &app.overlay else {
            panic!("attributes dialog");
        };
        assert_eq!(dialog.field, field);
    }
}

#[test]
fn quit_buttons_are_yes_and_no() {
    let dir = TempDir::new().expect("temp");
    let mut app = app_at(&dir);
    app.set_overlay(Overlay::QuitConfirm(Confirm::default()));
    assert_eq!(code(&mut app, 0), KeyCode::Char('y'));
    assert_eq!(code(&mut app, 1), KeyCode::Char('n'));
}

#[test]
fn delete_buttons_are_yes_and_escape() {
    let dir = TempDir::new().expect("temp");
    let mut app = app_at(&dir);
    app.set_overlay(Overlay::DeleteConfirm(Vec::new()));
    assert_eq!(code(&mut app, 0), KeyCode::Char('y'));
    assert_eq!(code(&mut app, 1), KeyCode::Esc);
}

/// The name dialog has a cursor, so both its buttons are Enter — the click
/// moves the cursor to the button it landed on first, and Enter on Cancel
/// cancels. The plain input box has no cursor, so Cancel is still Escape.
#[test]
fn name_and_new_file_buttons_press_the_button_they_land_on() {
    use crate::models::NameField;

    let dir = TempDir::new().expect("temp");
    let mut app = app_at(&dir);

    app.set_overlay(Overlay::NamePrompt(NamePrompt::make_folder()));
    assert_eq!(code(&mut app, 0), KeyCode::Enter);
    let Overlay::NamePrompt(prompt) = &app.overlay else {
        panic!("name prompt");
    };
    assert_eq!(prompt.field(), NameField::Ok);

    assert_eq!(code(&mut app, 1), KeyCode::Enter);
    let Overlay::NamePrompt(prompt) = &app.overlay else {
        panic!("name prompt");
    };
    assert_eq!(
        prompt.field(),
        NameField::Cancel,
        "a click on Cancel must move the cursor there, or Enter would act"
    );

    app.set_overlay(Overlay::NewFileInput("note.txt".into()));
    assert_eq!(code(&mut app, 0), KeyCode::Enter);
    assert_eq!(code(&mut app, 1), KeyCode::Esc);
}

#[test]
fn copy_buttons_name_copy_and_cancel_and_leave_the_disabled_filter_alone() {
    let dir = TempDir::new().expect("temp");
    let mut app = app_at(&dir);
    app.set_overlay(Overlay::CopyPrompt(Box::new(CopyPrompt::new(
        "a.txt",
        Path::new("/tmp"),
    ))));
    assert_eq!(code(&mut app, 0), KeyCode::Enter);
    let Overlay::CopyPrompt(prompt) = &app.overlay else {
        panic!("copy dialog");
    };
    assert_eq!(prompt.field, CopyField::Copy);
    assert!(dialog_button_key(&mut app, 1).is_none());
    assert_eq!(code(&mut app, 2), KeyCode::Esc);
}

#[test]
fn a_help_overlay_has_no_buttons_to_click() {
    let dir = TempDir::new().expect("temp");
    let mut app = app_at(&dir);
    app.set_overlay(Overlay::Help);
    assert!(dialog_button_key(&mut app, 0).is_none());
}

// ─── the remaining button maps ──────────────────────────────────────────────

#[test]
fn the_mask_dialog_buttons_name_the_button_they_land_on() {
    use crate::models::{MaskPrompt, MaskPurpose};

    let dir = TempDir::new().expect("temp");
    let mut app = app_at(&dir);
    app.set_overlay(Overlay::MaskPrompt(MaskPrompt::new(
        MaskPurpose::Select,
        "*",
    )));

    assert_eq!(code(&mut app, 0), KeyCode::Enter);
    let Overlay::MaskPrompt(prompt) = &app.overlay else {
        panic!("still open");
    };
    assert!(!prompt.on_cancel(), "a click on OK stays on OK");

    assert_eq!(code(&mut app, 1), KeyCode::Enter);
    let Overlay::MaskPrompt(prompt) = &app.overlay else {
        panic!("still open");
    };
    assert!(prompt.on_cancel(), "and a click on Cancel moves there");
}

#[test]
fn the_information_panels_only_button_dismisses_it() {
    let dir = TempDir::new().expect("temp");
    let mut app = app_at(&dir);
    app.set_overlay(Overlay::Information(Box::default()));
    assert_eq!(code(&mut app, 0), KeyCode::Esc);
    app.set_overlay(Overlay::Help);
    assert!(dialog_button_key(&mut app, 0).is_none());
    app.set_overlay(Overlay::Editor(Editor::default()));
    assert!(dialog_button_key(&mut app, 0).is_none());
}

#[test]
fn find_file_session_cleanup_prompts_and_stubs_map_their_buttons() {
    use crate::models::*;
    let dir = TempDir::new().expect("temp");
    let mut app = app_at(&dir);
    app.set_overlay(Overlay::FindFile(Box::<FindFile>::default()));
    assert_eq!(code(&mut app, 1), KeyCode::Enter);
    assert!(matches!(&app.overlay, Overlay::FindFile(d) if d.field() == FindFileField::Cancel));
    app.set_overlay(Overlay::SessionEdit(Box::new(SessionEdit::create())));
    assert_eq!(code(&mut app, 1), KeyCode::Enter);
    assert!(matches!(&app.overlay, Overlay::SessionEdit(d) if d.field() == SessionField::Cancel));
    app.set_overlay(Overlay::DiskCleanup(Box::<DiskCleanupDialog>::default()));
    assert_eq!(code(&mut app, 0), KeyCode::Enter);
    assert!(matches!(&app.overlay, Overlay::DiskCleanup(d) if d.field == CleanupField::Run));
    let prompt = InputPrompt::new(InputPurpose::ImportNetBox, "");
    app.set_overlay(Overlay::InputPrompt(Box::new(prompt)));
    assert_eq!(code(&mut app, 1), KeyCode::Enter);
    assert!(matches!(&app.overlay, Overlay::InputPrompt(p) if p.on_cancel()));
    let q = Question::new(QuestionPurpose::DeleteSession(
        SessionName::new("s").unwrap(),
    ));
    app.set_overlay(Overlay::Question(q));
    assert_eq!(
        (code(&mut app, 0), code(&mut app, 1)),
        (KeyCode::Char('y'), KeyCode::Char('n'))
    );
    app.set_overlay(Overlay::Filter(FilterPrompt::default()));
    assert_eq!(code(&mut app, 0), KeyCode::Enter);
}
