//! Purpose: Copy and paste without a system clipboard or an editor, and a folder reloaded as text.

use super::*;

fn editor_app(content: &str, path: std::path::PathBuf) -> App {
    let mut app = App::new().expect("app");
    app.set_overlay(Overlay::Editor(Editor::from_content(
        "t.txt".to_owned(),
        path,
        content,
    )));
    app
}

#[test]
fn copying_falls_back_to_the_terminal_clipboard() {
    let dir = TempDir::new().expect("dir");
    let mut app = editor_app("ab", dir.path().join("t.txt"));
    app.clipboard.install(Box::new(OscClipboard));
    app.get_editor_mut().expect("editor").select_all();
    crate::ops::copy_editor_selection(&mut app);
    let notice = app.get_editor().expect("editor").notice.clone();
    assert!(
        notice
            .as_deref()
            .is_some_and(|text| text.contains("terminal")),
        "{notice:?}"
    );
}

#[test]
fn the_file_name_is_typed_even_without_a_clipboard() {
    let dir = TempDir::new().expect("dir");
    let path = dir.path().join("t.txt");
    let mut app = editor_app("", path.clone());
    crate::ops::insert_editor_file_name(&mut app);
    let editor = app.get_editor().expect("editor");
    let name = crate::path_display::label(&path).into_owned();
    assert_eq!(editor.content(), name);
    let notice = editor.notice.as_deref().unwrap_or_default();
    assert!(notice.starts_with("Copy failed"), "{notice}");
}

#[test]
fn paste_and_copy_do_nothing_without_an_editor() {
    let mut app = App::new().expect("app");
    crate::ops::copy_editor_selection(&mut app);
    crate::ops::insert_editor_file_name(&mut app);
    crate::ops::paste_into_editor(&mut app);
    assert!(app.get_editor().is_none());
}

#[test]
fn reloading_a_directory_as_text_is_refused() {
    let dir = TempDir::new().expect("dir");
    let mut app = editor_app("x", dir.path().to_path_buf());
    crate::ops::reload_editor_encoding(&mut app, crate::encoding::TextEncoding::Utf8);
    let notice = app.get_editor().expect("editor").notice.clone();
    assert!(notice.is_some(), "{notice:?}");
}

#[test]
fn reloading_rejects_nul_bytes_and_preserves_indent_and_last_search() {
    let dir = TempDir::new().expect("dir");
    let path = dir.path().join("sample.txt");
    fs::write(&path, b"a\x00b").expect("write binary");
    let mut app = editor_app("initial", path.clone());
    {
        let editor = app.get_editor_mut().expect("editor");
        editor.format.indent = Indent::Tabs;
        editor.search.last = crate::search::Needle::new("needle").map(crate::search::Search::new);
    }
    crate::ops::reload_editor_encoding(&mut app, crate::encoding::TextEncoding::Windows1251);
    let editor = app.get_editor().expect("editor");
    assert_eq!(editor.lines, ["initial"]);
    assert!(editor
        .notice
        .as_deref()
        .is_some_and(|msg| msg.contains("NUL")));

    fs::write(&path, b"reloaded text").expect("write text");
    crate::ops::reload_editor_encoding(&mut app, crate::encoding::TextEncoding::Utf8);
    let editor = app.get_editor().expect("editor");
    assert_eq!(editor.lines, ["reloaded text"]);
    assert_eq!(editor.format.indent, Indent::Tabs);
    assert_eq!(
        editor.search.last.as_ref().map(|s| s.needle.as_str()),
        Some("needle")
    );
}
