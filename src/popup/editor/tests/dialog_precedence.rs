//! Purpose: an open editor dialog takes the keys first — clipboard and save shortcuts cannot reach the document behind search or exit confirmation.

use super::*;

#[test]
fn clipboard_shortcuts_are_consumed_by_search_and_exit_dialogs() {
    let temp = TempDir::new().expect("temporary directory");
    let mut app = app_with_editor("text", temp.path().join("text.txt"));
    let clipboard = Clipboard::default();
    app.clipboard.install(Box::new(clipboard.clone()));
    key(&mut app, KeyCode::Right, KeyModifiers::SHIFT);
    key(&mut app, KeyCode::F(7), KeyModifiers::NONE);
    key(&mut app, KeyCode::Insert, KeyModifiers::CONTROL);
    key(&mut app, KeyCode::Insert, KeyModifiers::SHIFT);
    assert_eq!(clipboard.0.borrow().reads, 0);
    assert_eq!(clipboard.0.borrow().writes, 0);
    key(&mut app, KeyCode::Esc, KeyModifiers::NONE);
    app.get_editor_mut().expect("editor").state = EditState::ConfirmingExit(ExitChoice::Save);
    key(&mut app, KeyCode::Insert, KeyModifiers::CONTROL);
    key(&mut app, KeyCode::Insert, KeyModifiers::SHIFT);
    assert_eq!(clipboard.0.borrow().reads, 0);
    assert_eq!(clipboard.0.borrow().writes, 0);
    assert!(matches!(app.overlay, Overlay::Editor(_)));
}

#[test]
fn save_shortcut_cannot_escape_search_or_exit_confirmation() {
    let temp = TempDir::new().expect("temporary directory");
    let path = temp.path().join("text.txt");
    std::fs::write(&path, "original界").expect("original file");
    let mut app = app_with_editor("original界", path.clone());
    key(&mut app, KeyCode::End, KeyModifiers::NONE);
    key(&mut app, KeyCode::Char('!'), KeyModifiers::NONE);

    key(&mut app, KeyCode::F(7), KeyModifiers::NONE);
    key(&mut app, KeyCode::Char('界'), KeyModifiers::NONE);
    key(&mut app, KeyCode::F(2), KeyModifiers::NONE);
    assert_unsaved_search(&app, &path);

    key(&mut app, KeyCode::Esc, KeyModifiers::NONE); // Close search.
    key(&mut app, KeyCode::Esc, KeyModifiers::NONE); // Ask whether to save.
    key(&mut app, KeyCode::F(2), KeyModifiers::NONE);
    let editor = app.get_editor().expect("confirmation remains in editor");
    assert!(editor.search.prompt.is_none());
    assert_eq!(editor.state, EditState::ConfirmingExit(ExitChoice::Save));
    assert_eq!(editor.lines, ["original界!"]);
    assert_eq!(
        std::fs::read_to_string(path).expect("disk content"),
        "original界"
    );
}

#[test]
fn ignored_editor_command_keeps_selection_and_unsaved_file_unchanged() {
    let temp = TempDir::new().expect("temporary directory");
    let path = temp.path().join("text.txt");
    std::fs::write(&path, "界text").expect("original file");
    let mut app = app_with_editor("界text", path.clone());
    key(&mut app, KeyCode::End, KeyModifiers::NONE);
    key(&mut app, KeyCode::Char('!'), KeyModifiers::NONE);
    key(&mut app, KeyCode::Home, KeyModifiers::NONE);
    key(&mut app, KeyCode::Right, KeyModifiers::SHIFT);

    key(&mut app, KeyCode::F(8), KeyModifiers::NONE);
    let editor = app
        .get_editor()
        .expect("unsupported key stays inside editor");
    assert_eq!(editor.selected_text().as_deref(), Some("界"));
    assert_eq!(editor.lines, ["界text!"]);
    assert_eq!(editor.state, EditState::Modified);
    assert_eq!(
        std::fs::read_to_string(path).expect("file remains on disk"),
        "界text"
    );
}

/// `request_save` is only ever called from the `Editor` overlay's own key
/// handling, so its "no editor open" branch cannot be reached through
/// `process_popup_input`. Called directly, like the guard it is.
#[test]
fn a_save_request_does_nothing_without_an_open_editor() {
    let mut app = App::new().expect("test app");
    let before = app.status_text();

    save::request_save(&mut app, AfterSave::Close);

    assert!(app.get_editor().is_none());
    assert_eq!(
        app.status_text(),
        before,
        "the guard must not report anything"
    );
}

fn assert_unsaved_search(app: &App, path: &std::path::Path) {
    let editor = app.get_editor().expect("search remains in editor");
    assert_eq!(
        editor
            .search
            .prompt
            .as_ref()
            .expect("search stays open")
            .query,
        "界"
    );
    assert_eq!(editor.lines, ["original界!"]);
    assert_eq!(editor.state, EditState::Modified);
    assert_eq!(
        std::fs::read_to_string(path).expect("disk content"),
        "original界"
    );
}

#[test]
fn encoding_and_search_dialogs_take_keyboard_precedence_matching_render_order() {
    let temp = TempDir::new().expect("temporary directory");
    let mut app = app_with_editor("text", temp.path().join("text.txt"));
    {
        let editor = app.get_editor_mut().expect("editor");
        editor.state = EditState::ConfirmingExit(ExitChoice::Save);
        editor.encoding_choice = Some(TextEncoding::Utf8);
        assert_eq!(
            editor.modal(),
            Some(crate::models::EditorModal::Encoding(TextEncoding::Utf8))
        );
    }
    key(&mut app, KeyCode::Esc, KeyModifiers::NONE);
    let editor = app.get_editor().expect("editor stays open");
    assert_eq!(
        editor.modal(),
        Some(crate::models::EditorModal::ConfirmExit(ExitChoice::Save))
    );
}
