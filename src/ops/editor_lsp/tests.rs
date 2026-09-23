//! Purpose: Follow definitions without losing changes or accepting invalid target positions.

use super::*;
use crate::{
    lsp::positions::{Position, Range},
    models::Panel,
};
use tempfile::TempDir;

fn fixture(root: &TempDir) -> (App, Definition) {
    let panel = || Panel::from_entries(root.path().to_path_buf(), Vec::new());
    let mut app = App::with_panels(panel(), panel());
    let path = root.path().join("target.rs");
    std::fs::write(&path, "😀 fn function() {}\n").unwrap();
    let target = Definition {
        path,
        range: Range {
            start: Position {
                line: 0,
                character: 6,
            },
            end: Position {
                line: 0,
                character: 14,
            },
        },
    };
    app.set_overlay(Overlay::Editor(Editor::from_content(
        "source".into(),
        root.path().join("source.rs"),
        "source",
    )));
    (app, target)
}

#[test]
fn another_file_opens_at_the_utf16_target() {
    let root = TempDir::new().unwrap();
    let (mut app, target) = fixture(&root);
    definition(&mut app, &target);
    let editor = app.get_editor().unwrap();
    assert_eq!(editor.path, target.path);
    assert_eq!((editor.cursor_line, editor.cursor_col), (0, 5));
    definition(&mut app, &target);
    assert_eq!(app.get_editor().unwrap().cursor_col, 5);
}

#[test]
fn modified_source_and_invalid_targets_remain_open_with_an_explanation() {
    let root = TempDir::new().unwrap();
    let (mut app, target) = fixture(&root);
    app.get_editor_mut().unwrap().insert_char('x');
    definition(&mut app, &target);
    assert_eq!(
        app.get_editor().unwrap().path,
        root.path().join("source.rs")
    );
    assert_eq!(app.get_editor().unwrap().content(), "xsource");
    assert!(app
        .get_editor()
        .unwrap()
        .notice
        .as_ref()
        .unwrap()
        .contains("Save"));
}

#[test]
fn invalid_definition_targets_leave_the_source_open_with_an_explanation() {
    let root = TempDir::new().unwrap();
    let (mut app, mut target) = fixture(&root);
    app.get_editor_mut().unwrap().save().unwrap();
    target.range.start.character = 1;
    definition(&mut app, &target);
    assert_eq!(
        app.get_editor().unwrap().notice.as_deref(),
        Some("Invalid definition position")
    );
    target.path = root.path().join("missing.rs");
    definition(&mut app, &target);
    assert_eq!(
        app.get_editor().unwrap().notice.as_deref(),
        Some("Cannot read definition target")
    );
    target.path = root.path().to_path_buf();
    definition(&mut app, &target);
    assert_eq!(
        app.get_editor().unwrap().notice.as_deref(),
        Some("Definition target is not an editable text file")
    );
}

#[test]
fn empty_definition_targets_keep_the_configured_encoding() {
    let root = TempDir::new().unwrap();
    let (mut app, mut target) = fixture(&root);
    std::fs::write(&target.path, "").unwrap();
    target.range = Range::default();
    app.settings
        .set(crate::menu::SettingsPage::CodePages, "Windows-1251")
        .unwrap();
    let expected = crate::editor::editor_for(
        &app,
        target.path.clone(),
        "",
        crate::encoding::TextEncoding::Utf8,
    );
    assert_eq!(
        expected.format.encoding,
        crate::encoding::TextEncoding::Windows1251
    );
    definition(&mut app, &target);
    assert_eq!(
        app.get_editor().unwrap().format.encoding,
        crate::encoding::TextEncoding::Windows1251
    );
}
