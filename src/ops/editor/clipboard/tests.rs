//! Purpose: Clipboard fallback notice when no clipboard backend is installed.

use super::paste_into_editor;
use crate::models::{App, Editor, Overlay, Panel};

#[test]
fn paste_without_clipboard_backend_sets_notice() {
    let temp = tempfile::TempDir::new().expect("temp");
    let panel = Panel::from_entries(temp.path().to_path_buf(), Vec::new());
    let mut app = App::with_panels(panel.clone(), panel);
    app.set_overlay(Overlay::Editor(Editor::from_content(
        "note.txt".into(),
        temp.path().join("note.txt"),
        "hello",
    )));

    paste_into_editor(&mut app);

    let editor = app.get_editor().expect("editor stays open");
    assert_eq!(editor.content(), "hello");
    assert_eq!(
        editor.notice.as_deref(),
        Some("Paste failed: clipboard text is unavailable")
    );
}
