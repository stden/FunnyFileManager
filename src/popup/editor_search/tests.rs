//! Purpose: Search submit guards are no-ops when the editor or prompt has closed.

use super::submit_search;
use crate::models::{App, Editor, Overlay, Panel};
use std::path::PathBuf;

#[test]
fn submit_search_ignores_missing_editor_or_missing_prompt() {
    let empty = || Panel::from_entries(PathBuf::from("/tmp"), Vec::new());
    let mut app = App::with_panels(empty(), empty());
    submit_search(&mut app);
    assert!(matches!(app.overlay, Overlay::None));

    app.set_overlay(Overlay::Editor(Editor::default()));
    submit_search(&mut app);
    assert!(app.get_editor().expect("editor").search.prompt.is_none());
}
