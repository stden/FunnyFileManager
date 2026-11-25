//! Purpose: the editor's Ctrl+X and Shift+Del — cutting a selection to the clipboard, or preserving text when the clipboard fails.

use super::*;

#[test]
fn ctrl_x_and_shift_del_cut_selection_and_support_russian_layout() {
    let temp = TempDir::new().expect("temporary directory");
    let mut app = app_with_editor("a界\r\nβ🙂\r\nz!", temp.path().join("text.txt"));
    let clipboard = Clipboard::default();
    app.clipboard.install(Box::new(clipboard.clone()));
    key(&mut app, KeyCode::Right, KeyModifiers::SHIFT);
    key(&mut app, KeyCode::Right, KeyModifiers::SHIFT);
    key(&mut app, KeyCode::Char('x'), KeyModifiers::CONTROL);
    assert_eq!(clipboard.0.borrow().text, "a界");
    let editor = app.get_editor().expect("editor");
    assert_eq!(editor.lines, ["", "β🙂", "z!"]);
    assert_eq!(editor.selection, EditorSelection::None);
    assert_eq!(editor.state, EditState::Modified);
    assert_eq!(editor.notice.as_deref(), Some("Selection cut"));

    key(&mut app, KeyCode::Down, KeyModifiers::NONE);
    key(&mut app, KeyCode::Right, KeyModifiers::SHIFT);
    key(&mut app, KeyCode::Delete, KeyModifiers::SHIFT);
    assert_eq!(clipboard.0.borrow().text, "β");
    assert_eq!(app.get_editor().expect("editor").lines, ["", "🙂", "z!"]);

    key(&mut app, KeyCode::Right, KeyModifiers::SHIFT);
    key(&mut app, KeyCode::Char('ч'), KeyModifiers::CONTROL);
    assert_eq!(clipboard.0.borrow().text, "🙂");
    assert_eq!(app.get_editor().expect("editor").lines, ["", "", "z!"]);
}

#[test]
fn ctrl_a_then_ctrl_x_cuts_the_entire_document() {
    let temp = TempDir::new().expect("temporary directory");
    let mut app = app_with_editor("a界\r\nβ🙂\r\n", temp.path().join("text.txt"));
    let clipboard = Clipboard::default();
    app.clipboard.install(Box::new(clipboard.clone()));
    key(&mut app, KeyCode::Char('a'), KeyModifiers::CONTROL);
    key(&mut app, KeyCode::Char('x'), KeyModifiers::CONTROL);
    assert_eq!(clipboard.0.borrow().text, "a界\r\nβ🙂\r\n");
    let editor = app.get_editor().expect("editor");
    assert_eq!(editor.content(), "");
    assert_eq!(editor.selection, EditorSelection::None);
    assert_eq!(editor.state, EditState::Modified);
    assert_eq!(editor.notice.as_deref(), Some("Selection cut"));
    key(&mut app, KeyCode::Char('v'), KeyModifiers::CONTROL);
    assert_eq!(
        app.get_editor().expect("editor").content(),
        "a界\r\nβ🙂\r\n"
    );
}

#[test]
fn cut_without_selection_does_not_touch_the_clipboard_or_buffer() {
    let temp = TempDir::new().expect("temporary directory");
    let mut app = app_with_editor("text", temp.path().join("text.txt"));
    let clipboard = Clipboard::default();
    app.clipboard.install(Box::new(clipboard.clone()));
    key(&mut app, KeyCode::Char('x'), KeyModifiers::CONTROL);
    key(&mut app, KeyCode::Delete, KeyModifiers::SHIFT);
    assert_eq!(clipboard.0.borrow().writes, 0);
    let editor = app.get_editor().expect("editor");
    assert_eq!(editor.lines, ["text"]);
    assert_eq!(editor.state, EditState::Clean);
    assert_eq!(editor.notice, None);
}

#[test]
fn failed_cut_preserves_selection_and_buffer_and_reports_in_editor() {
    let temp = TempDir::new().expect("temporary directory");
    let mut app = app_with_editor("界🙂", temp.path().join("text.txt"));
    let clipboard = Clipboard::default();
    clipboard.0.borrow_mut().fail = true;
    app.clipboard.install(Box::new(clipboard.clone()));
    key(&mut app, KeyCode::Right, KeyModifiers::SHIFT);
    key(&mut app, KeyCode::Char('x'), KeyModifiers::CONTROL);
    let editor = app.get_editor().expect("editor");
    assert_eq!(editor.lines, ["界🙂"]);
    assert_eq!(editor.selected_text().as_deref(), Some("界"));
    assert_eq!(editor.state, EditState::Clean);
    let notice = editor.notice.as_deref().expect("visible error");
    assert!(notice.starts_with("Cut failed: "), "{notice}");
    assert_eq!(clipboard.0.borrow().text, "");
}

#[cfg(feature = "keepass")]
#[test]
fn editor_cut_cancels_the_pending_secret_wipe() {
    let temp = TempDir::new().expect("temporary directory");
    let mut app = app_with_editor("text", temp.path().join("text.txt"));
    app.clipboard.install(Box::new(Clipboard::default()));
    app.clipboard.arm(std::time::Instant::now());
    key(&mut app, KeyCode::Right, KeyModifiers::SHIFT);
    key(&mut app, KeyCode::Char('x'), KeyModifiers::CONTROL);
    assert!(!app.clipboard.is_armed());
}
