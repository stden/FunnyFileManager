//! Purpose: the wheel over an overlay — the viewer's and the editor's own
//! scroll command, or the Up and Down keys of a dialog's protocol.

use super::*;
use crate::models::{Viewer, Viewport};
use crate::test_support::task_list::{named, TaskApp};

#[test]
fn the_wheel_is_one_of_two_turns() {
    assert_eq!(Wheel::of(MouseEventKind::ScrollUp), Some(Wheel::Up));
    assert_eq!(Wheel::of(MouseEventKind::ScrollDown), Some(Wheel::Down));
    assert_eq!(Wheel::of(MouseEventKind::Moved), None);
}

#[test]
fn the_wheel_scrolls_the_viewer_a_line_at_a_time() {
    let dir = TempDir::new().expect("temp");
    let mut app = app_at(&dir);
    let mut term = terminal();
    let lines = (0..100).map(|i| format!("line {i}")).collect();
    app.set_overlay(Overlay::Viewer(Viewer::report("big.txt", lines)));
    app.set_viewport(Viewport::new(24));

    gesture(&mut term, &mut app, MouseEventKind::ScrollDown, (40, 12));
    gesture(&mut term, &mut app, MouseEventKind::ScrollDown, (40, 12));
    assert_eq!(app.get_viewer().expect("viewer").scroll.lines(), 2);
    gesture(&mut term, &mut app, MouseEventKind::ScrollUp, (40, 12));
    assert_eq!(app.get_viewer().expect("viewer").scroll.lines(), 1);
}

/// Over the task list's table the wheel is its cursor move; with its Sort by
/// menu open it is that menu's key, which the wheel routine hands back.
#[test]
fn the_wheel_is_the_task_lists_own_only_over_its_table() {
    let mut tasks = TaskApp::open(
        vec![named(10, "vim"), named(20, "python3")],
        crate::prompt::Privilege::User,
    );
    assert!(popup::wheel(
        &mut tasks.app,
        Wheel::Down,
        KeyModifiers::empty()
    ));
    assert_eq!(tasks.list().cursor().expect("a cursor").index(), 1);

    popup::process_popup_input(&mut tasks.app, KeyEvent::from(KeyCode::F(6)));
    assert!(matches!(
        tasks.list().modal(),
        crate::models::Modal::Menu(_)
    ));
    assert!(!popup::wheel(
        &mut tasks.app,
        Wheel::Down,
        KeyModifiers::empty()
    ));
}

/// Over the text the wheel moves the cursor a line, as Up and Down do; while
/// the exit question is up it is that dialog's key, which moves nothing.
#[test]
fn the_wheel_moves_the_editors_cursor_but_not_behind_its_dialog() {
    let dir = TempDir::new().expect("temp");
    let mut app = app_at(&dir);
    let mut term = terminal();
    let editor = Editor::from_content(
        "text".to_owned(),
        dir.path().join("text"),
        "one\ntwo\nthree",
    );
    app.set_overlay(Overlay::Editor(editor));

    gesture(&mut term, &mut app, MouseEventKind::ScrollDown, (40, 12));
    assert_eq!(app.get_editor_mut().expect("editor").cursor_line, 1);

    let editor = app.get_editor_mut().expect("editor");
    editor.state = EditState::ConfirmingExit(ExitChoice::Save);
    gesture(&mut term, &mut app, MouseEventKind::ScrollDown, (40, 12));
    let editor = app.get_editor_mut().expect("editor");
    assert_eq!(editor.cursor_line, 1, "the exit question read the key");
    assert_eq!(editor.state.exit_choice(), Some(ExitChoice::Save));
}

/// Shift held with the wheel selects in the editor, as Shift with Down does;
/// the plain wheel only moves the caret.
#[test]
fn shift_with_the_wheel_selects_in_the_editor() {
    let dir = TempDir::new().expect("temp");
    let mut app = app_at(&dir);
    let mut term = terminal();
    let editor = Editor::from_content(
        "text".to_owned(),
        dir.path().join("text"),
        "one\ntwo\nthree",
    );
    app.set_overlay(Overlay::Editor(editor));

    dispatch_dialog_mouse(
        &mut term,
        &mut app,
        mouse_with(MouseEventKind::ScrollDown, 40, 12, KeyModifiers::empty()),
    );
    let editor = app.get_editor_mut().expect("editor");
    assert_eq!(editor.cursor_line, 1);
    assert!(editor.selection_range().is_none(), "a plain wheel selected");

    dispatch_dialog_mouse(
        &mut term,
        &mut app,
        mouse_with(MouseEventKind::ScrollDown, 40, 12, KeyModifiers::SHIFT),
    );
    let editor = app.get_editor_mut().expect("editor");
    assert_eq!(editor.cursor_line, 2);
    assert!(
        editor.selection_range().is_some(),
        "Shift with the wheel did not select"
    );
}
