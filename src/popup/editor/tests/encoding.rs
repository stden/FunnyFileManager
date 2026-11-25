//! Purpose: the editor's encoding dialog (Shift+F8) — opening and closing it, walking the list, and the keys that are not a choice — and F8, which toggles OEM-866 and Windows-1251 without a dialog.

use super::*;

#[test]
fn shift_f8_opens_the_encoding_dialog_and_esc_closes_it() {
    let temp = TempDir::new().expect("temporary directory");
    let mut app = app_with_editor("text", temp.path().join("text.txt"));
    key(&mut app, KeyCode::F(8), KeyModifiers::SHIFT);
    assert_eq!(
        app.get_editor().expect("editor").encoding_choice,
        Some(TextEncoding::Utf8)
    );
    key(&mut app, KeyCode::Esc, KeyModifiers::NONE);
    assert_eq!(app.get_editor().expect("editor").encoding_choice, None);
}

#[test]
fn f8_toggles_the_code_page_without_a_dialog_and_keeps_the_caret() {
    let temp = TempDir::new().expect("temporary directory");
    let path = temp.path().join("text.txt");
    std::fs::write(&path, [0x8f, 0xe0, 0xa8, 0xa2, 0xa5, 0xe2]).expect("file");
    let mut app = app_with_editor("text", path);
    app.get_editor_mut().expect("editor").cursor_col = 2;
    key(&mut app, KeyCode::F(8), KeyModifiers::NONE);
    let editor = app.get_editor().expect("editor");
    assert_eq!(editor.encoding_choice, None);
    assert_eq!(editor.format.encoding, TextEncoding::Oem866);
    assert_eq!(editor.content(), "Привет");
    assert_eq!((editor.cursor_line, editor.cursor_col), (0, 2));
    assert_eq!(editor.state, EditState::Clean);
    key(&mut app, KeyCode::F(8), KeyModifiers::NONE);
    let editor = app.get_editor().expect("editor");
    assert_eq!(editor.format.encoding, TextEncoding::Windows1251);
    assert_eq!(editor.content(), "ЏаЁўҐв");
}

#[test]
fn f8_refuses_to_reread_the_file_over_unsaved_text() {
    let temp = TempDir::new().expect("temporary directory");
    let path = temp.path().join("text.txt");
    std::fs::write(&path, "text").expect("file");
    let mut app = app_with_editor("text", path);
    app.get_editor_mut().expect("editor").insert_char('!');
    key(&mut app, KeyCode::F(8), KeyModifiers::NONE);
    let editor = app.get_editor().expect("editor");
    assert_eq!(editor.format.encoding, TextEncoding::Utf8);
    assert_eq!(editor.content(), "!text");
    assert_eq!(
        editor.notice.as_deref(),
        Some("Save or discard changes before reloading the file")
    );
}

#[test]
fn encoding_arrows_walk_the_list_and_enter_keeps_utf8() {
    let temp = TempDir::new().expect("temporary directory");
    let path = temp.path().join("text.txt");
    std::fs::write(&path, "text").expect("file");
    let mut app = app_with_editor("text", path);
    key(&mut app, KeyCode::F(8), KeyModifiers::SHIFT);
    key(&mut app, KeyCode::Left, KeyModifiers::NONE);
    key(&mut app, KeyCode::Right, KeyModifiers::NONE);
    key(&mut app, KeyCode::Tab, KeyModifiers::NONE);
    key(&mut app, KeyCode::Up, KeyModifiers::NONE);
    key(&mut app, KeyCode::Enter, KeyModifiers::NONE);
    let editor = app.get_editor().expect("editor");
    assert_eq!(editor.encoding_choice, None);
    assert_eq!(editor.format.encoding, TextEncoding::Utf8);
}

#[test]
fn encoding_helpers_do_nothing_without_an_open_editor() {
    let mut app = App::new().expect("test app");
    process_encoding(&mut app, KeyCode::Enter);
    crate::ops::change_editor_encoding(&mut app, TextEncoding::Utf8);
    crate::ops::reload_editor_encoding(&mut app, TextEncoding::Utf8);
    crate::ops::toggle_editor_code_page(&mut app);
    assert!(app.get_editor().is_none());
}

#[test]
fn process_encoding_returns_when_the_dialog_is_not_open() {
    let temp = TempDir::new().expect("temporary directory");
    let mut app = app_with_editor("text", temp.path().join("text.txt"));
    process_encoding(&mut app, KeyCode::Down);
    assert_eq!(app.get_editor().expect("editor").encoding_choice, None);
}

#[test]
fn encoding_reload_of_a_missing_file_leaves_a_notice() {
    let temp = TempDir::new().expect("temporary directory");
    let mut app = app_with_editor("text", temp.path().join("missing.txt"));
    key(&mut app, KeyCode::F(8), KeyModifiers::SHIFT);
    key(&mut app, KeyCode::Char('r'), KeyModifiers::NONE);
    let editor = app.get_editor().expect("editor");
    assert!(editor.notice.is_some(), "{:?}", editor.notice);
}

#[test]
fn encoding_dialog_ignores_keys_that_are_not_choices() {
    let temp = TempDir::new().expect("temporary directory");
    let mut app = app_with_editor("text", temp.path().join("text.txt"));
    key(&mut app, KeyCode::F(8), KeyModifiers::SHIFT);
    key(&mut app, KeyCode::Char('x'), KeyModifiers::NONE);
    let editor = app.get_editor().expect("editor");
    assert_eq!(editor.encoding_choice, Some(TextEncoding::Utf8));
    assert_eq!(
        editor.lines,
        ["text"],
        "the key must not reach the document"
    );
}
