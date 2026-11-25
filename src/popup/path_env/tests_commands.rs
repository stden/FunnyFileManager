//! Purpose: Direct PATH commands share browse effects and respect nested dialogs.

use super::{
    tests::{add, dialog, fixture},
    *,
};

#[test]
fn typed_close_checks_whether_the_path_draft_changed() {
    let (_temp, mut app) = fixture();
    apply(&mut app, PathCommand::Close);
    assert!(matches!(app.overlay, Overlay::None));

    let (temp, mut app) = fixture();
    add(&mut app, &temp.path().to_string_lossy());
    apply(&mut app, PathCommand::Close);
    assert!(matches!(dialog(&app).mode, Mode::ConfirmDiscard));
    assert_eq!(dialog(&app).entries.len(), 1);
}

#[test]
fn typed_browse_commands_leave_entry_fields_and_questions_untouched() {
    let (_temp, mut app) = fixture();
    apply(&mut app, PathCommand::Add);
    apply(&mut app, PathCommand::Close);
    assert!(matches!(dialog(&app).mode, Mode::Edit { .. }));
    assert!(process(&mut app, KeyCode::Esc, KeyModifiers::NONE));
    apply(&mut app, PathCommand::Save);
    apply(&mut app, PathCommand::Add);
    assert!(matches!(dialog(&app).mode, Mode::ConfirmSave));
    app.close_overlay();
    apply(&mut app, PathCommand::Save);
    assert!(matches!(app.overlay, Overlay::None));
}

#[test]
fn popup_dispatcher_routes_down_cursor_movement_in_path_editor() {
    let (_temp, mut app) = fixture();
    add(&mut app, "/first");
    add(&mut app, "/second");
    apply(&mut app, PathCommand::CursorUp);
    assert_eq!(dialog(&app).selected, 0);
    assert!(crate::popup::process_popup_input(
        &mut app,
        KeyEvent::new(KeyCode::Down, KeyModifiers::NONE),
    ));
    assert_eq!(dialog(&app).selected, 1);
}

#[test]
fn edit_on_empty_list_is_ignored_and_confirm_save_accepts_ok() {
    let (_temp, mut app) = fixture();
    apply(&mut app, PathCommand::Edit);
    assert!(matches!(dialog(&app).mode, Mode::Browse));

    let Overlay::PathEditor(editor) = &mut app.overlay else {
        panic!("PATH dialog");
    };
    editor.mode = Mode::ConfirmSave;
    confirm_save_with(editor, KeyCode::Enter, |_| Ok(()));
    assert_eq!(editor.message, "");
}
