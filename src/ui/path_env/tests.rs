use super::*;
use crate::{
    lineedit::LineEditor,
    path_env::{Scope, Snapshot},
};
use ratatui::{
    backend::{Backend, TestBackend},
    Terminal,
};

fn editing_path(text: &str) -> PathEditor {
    let mut editor = PathEditor::new(
        Scope::User,
        Snapshot {
            value: String::new(),
            revision: String::new(),
            location: "fixture".to_owned(),
        },
    );
    let mut input = LineEditor::new(text);
    input.left();
    editor.mode = Mode::Edit { index: None, input };
    editor
}

#[test]
fn a_long_path_scrolls_with_the_caret_including_wide_characters() {
    let temp = tempfile::TempDir::new().expect("tempdir");
    let text = format!("{}/very-long-folder-name/界/ending", temp.path().display());
    let editor = editing_path(&text);
    let mut terminal = Terminal::new(TestBackend::new(32, 14)).expect("terminal");
    terminal.draw(|f| render(f, &editor)).expect("draw");
    let screen: String = terminal
        .backend()
        .buffer()
        .content
        .iter()
        .map(ratatui::buffer::Cell::symbol)
        .collect();
    assert!(
        screen.contains("/ending") && screen.contains('界'),
        "caret tail must remain visible: {screen}"
    );
    let cursor = terminal
        .backend_mut()
        .get_cursor_position()
        .expect("cursor");
    assert_eq!(
        terminal.backend().buffer()[(cursor.x, cursor.y)].symbol(),
        "g"
    );
}

#[test]
fn invalid_path_rows_explain_why_each_directory_cannot_be_saved() {
    let temp = tempfile::TempDir::new().expect("tempdir");
    let file = temp.path().join("file");
    std::fs::write(&file, "fixture").expect("file");
    let mut editor = validation_fixture(&temp, &file);
    crate::ops::path_env::recheck(&mut editor);
    let mut terminal = Terminal::new(TestBackend::new(100, 30)).expect("terminal");
    terminal.draw(|f| render(f, &editor)).expect("draw");
    let screen: String = terminal
        .backend()
        .buffer()
        .content
        .iter()
        .map(ratatui::buffer::Cell::symbol)
        .collect();
    for explanation in [
        "OK",
        "MISSING",
        "NOT A DIRECTORY",
        "UNKNOWN VARIABLE",
        "NOT ABSOLUTE",
        "EMPTY",
    ] {
        assert!(
            screen.contains(explanation),
            "missing validation explanation: {explanation}"
        );
    }
}

fn validation_fixture(temp: &tempfile::TempDir, file: &std::path::Path) -> PathEditor {
    let unknown = if cfg!(windows) {
        "%FFM_MISSING_93287%"
    } else {
        "$FFM_MISSING_93287"
    };
    let entries = [
        temp.path().display().to_string(),
        file.display().to_string(),
        temp.path().join("absent").display().to_string(),
        unknown.to_owned(),
        "relative".to_owned(),
        String::new(),
    ];
    PathEditor::new(
        Scope::User,
        Snapshot {
            value: entries.join(&crate::path_env::separator().to_string()),
            revision: String::new(),
            location: "fixture".to_owned(),
        },
    )
}
