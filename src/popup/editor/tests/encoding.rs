//! Purpose: the editor's encoding dialog (F8) — opening and closing it, walking the list, and the keys that are not a choice.

use super::*;

#[test]
fn f8_opens_the_encoding_dialog_and_esc_closes_it() {
    let temp = TempDir::new().expect("temporary directory");
    let mut app = app_with_editor("text", temp.path().join("text.txt"));
    key(&mut app, KeyCode::F(8), KeyModifiers::NONE);
    assert_eq!(
        app.get_editor().expect("editor").encoding_choice,
        Some(TextEncoding::Utf8)
    );
    key(&mut app, KeyCode::Esc, KeyModifiers::NONE);
    assert_eq!(app.get_editor().expect("editor").encoding_choice, None);
}

#[test]
fn encoding_arrows_walk_the_list_and_enter_keeps_utf8() {
    let temp = TempDir::new().expect("temporary directory");
    let path = temp.path().join("text.txt");
    std::fs::write(&path, "text").expect("file");
    let mut app = app_with_editor("text", path);
    key(&mut app, KeyCode::F(8), KeyModifiers::NONE);
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
    key(&mut app, KeyCode::F(8), KeyModifiers::NONE);
    key(&mut app, KeyCode::Char('r'), KeyModifiers::NONE);
    let editor = app.get_editor().expect("editor");
    assert!(editor.notice.is_some(), "{:?}", editor.notice);
}

#[test]
fn encoding_dialog_ignores_keys_that_are_not_choices() {
    let temp = TempDir::new().expect("temporary directory");
    let mut app = app_with_editor("text", temp.path().join("text.txt"));
    key(&mut app, KeyCode::F(8), KeyModifiers::NONE);
    key(&mut app, KeyCode::Char('x'), KeyModifiers::NONE);
    let editor = app.get_editor().expect("editor");
    assert_eq!(editor.encoding_choice, Some(TextEncoding::Utf8));
    assert_eq!(
        editor.lines,
        ["text"],
        "the key must not reach the document"
    );
}
