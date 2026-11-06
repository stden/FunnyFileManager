//! Purpose: mouse clicks on the F4 editor body move the cursor to the clicked
//! line and column.

use super::super::editor::click_editor_body;
use super::*;

#[test]
fn clicking_editor_body_moves_cursor_to_that_line() {
    let dir = TempDir::new().expect("temp");
    let mut app = app_at(&dir);
    let mut term = terminal();
    let content = "line0\nline1\nline2\nline3\nline4\nline5\nline6\nline7\nline8\nline9";
    let editor = Editor::from_content("test".to_owned(), dir.path().join("test"), content);
    app.set_overlay(Overlay::Editor(editor));
    draw(&mut app, &mut term);
    gesture(
        &mut term,
        &mut app,
        MouseEventKind::Down(MouseButton::Left),
        (0, 5),
    );
    assert_eq!(
        app.get_editor().unwrap().cursor_line,
        4,
        "row 5 = body row 4 = document line 4 when scroll_y=0"
    );
}

#[test]
fn clicking_editor_chrome_or_a_tiny_terminal_preserves_the_cursor() {
    let dir = TempDir::new().expect("temp");
    let mut app = app_at(&dir);
    let mut editor = Editor::from_content("text".into(), dir.path().join("text"), "ab\ncd");
    editor.move_down(1);
    app.set_overlay(Overlay::Editor(editor));
    let term = terminal();
    assert!(click_editor_body(&term, &mut app, 0, 0));
    assert_eq!(app.get_editor().expect("editor").cursor_line, 1);
    let tiny = Terminal::new(WritableTestBackend::new(1, 1)).expect("terminal");
    assert!(click_editor_body(&tiny, &mut app, 0, 1));
    assert_eq!(app.get_editor().expect("editor").cursor_line, 1);
}
