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
