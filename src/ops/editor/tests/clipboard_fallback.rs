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
fn paste_and_copy_do_nothing_without_an_editor() {
    let mut app = App::new().expect("app");
    crate::ops::copy_editor_selection(&mut app);
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
