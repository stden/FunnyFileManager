//! Steps for F4 editor save, F7 search, Shift+F7 repeat search and not-found modal.

use super::BehaviorWorld;
use crate::models::EditorModal;
use cucumber::{gherkin::Step, given, then};

#[given(expr = "a multi-line file {string} with lines:")]
fn multiline_file(world: &mut BehaviorWorld, step: &Step, name: String) {
    let lines: Vec<&str> = step
        .table()
        .expect("data table of lines")
        .rows
        .iter()
        .map(|row| row[0].as_str())
        .collect();
    let case = world.case();
    std::fs::write(case.root.path().join("left").join(&name), lines.join("\n")).unwrap();
    case.app.left_panel.load_entries().unwrap();
    assert!(case.app.left_panel.select_named(&name));
    case.paint();
}

#[then(expr = "the editor caret sits at line {int} and column {int}")]
fn editor_caret(world: &mut BehaviorWorld, line: usize, col: usize) {
    let editor = world
        .case()
        .app
        .get_editor()
        .expect("editor must remain open");
    assert_eq!((editor.cursor_line, editor.cursor_col), (line, col));
}

#[then("the editor shows the not-found warning")]
fn editor_not_found(world: &mut BehaviorWorld) {
    let editor = world
        .case()
        .app
        .get_editor()
        .expect("editor must remain open");
    assert!(
        matches!(editor.modal(), Some(EditorModal::NotFound(_))),
        "expected NotFound modal"
    );
}
