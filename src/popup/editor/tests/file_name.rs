//! Purpose: the editor's Ctrl+F — the file's full name typed at the cursor and copied, or typed alone when the clipboard fails.

use super::*;

/// Far's Ctrl+F: the file's full name is typed at the cursor, replacing the
/// selection, and copied to the clipboard.
#[test]
fn ctrl_f_types_the_file_name_and_copies_it() {
    let temp = TempDir::new().expect("temporary directory");
    let path = temp.path().join("notes.txt");
    let mut app = app_with_editor("ab", path.clone());
    let clipboard = Clipboard::default();
    app.clipboard.install(Box::new(clipboard.clone()));
    key(&mut app, KeyCode::Right, KeyModifiers::SHIFT);
    key(&mut app, KeyCode::Char('f'), KeyModifiers::CONTROL);
    let name = crate::path_display::label(&path).into_owned();
    let editor = app.get_editor().expect("editor");
    assert_eq!(editor.lines, [format!("{name}b")]);
    assert_eq!(editor.state, EditState::Modified);
    assert_eq!(editor.notice.as_deref(), Some("File name copied"));
    assert_eq!(clipboard.0.borrow().text, name);
}

#[test]
fn ctrl_f_types_the_file_name_when_the_clipboard_fails() {
    let temp = TempDir::new().expect("temporary directory");
    let path = temp.path().join("notes.txt");
    let mut app = app_with_editor("", path.clone());
    let clipboard = Clipboard::default();
    clipboard.0.borrow_mut().fail = true;
    app.clipboard.install(Box::new(clipboard));
    key(&mut app, KeyCode::Char('f'), KeyModifiers::CONTROL);
    let editor = app.get_editor().expect("editor");
    assert_eq!(editor.content(), crate::path_display::label(&path));
    let notice = editor.notice.as_deref().expect("visible error");
    assert!(notice.starts_with("Copy failed: "), "{notice}");
}

#[test]
fn ctrl_alt_l_formats_json_and_xml_in_the_editor() {
    let temp = TempDir::new().expect("temporary directory");
    let mut app = app_with_editor("{\"a\":1,\"b\":[2,3]}\n", temp.path().join("config.json"));
    key(
        &mut app,
        KeyCode::Char('l'),
        KeyModifiers::CONTROL | KeyModifiers::ALT,
    );
    let editor = app.get_editor().expect("editor");
    assert_eq!(
        editor.content(),
        "{\n  \"a\": 1,\n  \"b\": [\n    2,\n    3\n  ]\n}\n"
    );
    assert_eq!(editor.state, EditState::Modified);
    assert_eq!(editor.notice.as_deref(), Some("Formatted JSON"));
}
