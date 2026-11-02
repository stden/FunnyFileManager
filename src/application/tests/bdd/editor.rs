//! F4 editor: open, text input, mouse body clicks, Far keys, F8 code-page toggle and Typst syntax.

use super::BehaviorWorld;
use crate::{encoding::TextEncoding, models::EditorModal};
use cucumber::{given, then, when};
use ratatui::style::Color;

#[then("the editor is open")]
fn editor_open(world: &mut BehaviorWorld) {
    assert!(
        world.case().app.get_editor().is_some(),
        "expected an Editor overlay to be active"
    );
}

#[then("the editor asks whether to save changes")]
fn confirm_exit(world: &mut BehaviorWorld) {
    let editor = world
        .case()
        .app
        .get_editor()
        .expect("the editor stays open");
    assert!(
        matches!(editor.modal(), Some(EditorModal::ConfirmExit(_))),
        "modified Esc must open the save confirmation"
    );
}

/// Inserts `text` at the current editor cursor position via a paste event.
#[when(expr = "I type the text {string}")]
fn type_text(world: &mut BehaviorWorld, text: String) {
    world.case().type_text(&text);
}

#[when(expr = "I left-click at column {int} and row {int}")]
fn left_click(world: &mut BehaviorWorld, column: u16, row: u16) {
    world.case().click(column, row);
}

#[then(expr = "editor line {int} equals {string}")]
fn editor_line_equals(world: &mut BehaviorWorld, index: usize, expected: String) {
    let editor = world
        .case()
        .app
        .get_editor()
        .expect("editor must stay open");
    assert_eq!(
        editor.lines.get(index).map(String::as_str),
        Some(expected.as_str())
    );
}

#[given(expr = "a Windows-1251 file named {string} encoding {string}")]
fn windows_1251_file(world: &mut BehaviorWorld, name: String, text: String) {
    let bytes = TextEncoding::Windows1251.encode(&text).unwrap();
    let case = world.case();
    std::fs::write(case.root.path().join("left").join(&name), bytes).unwrap();
    case.app.left_panel.load_entries().unwrap();
    assert!(case.app.left_panel.select_named(&name));
    case.paint();
}

#[then("the editor screen paints Typst directive string and comment colors")]
fn typst_colors_painted(world: &mut BehaviorWorld) {
    let screen = &world.case().screen;
    assert!(
        screen.contains_color(Color::Rgb(0, 255, 0)),
        "expected directive green on #set line"
    );
    assert!(
        screen.contains_color(Color::Rgb(255, 255, 0)),
        "expected string yellow on \"Hello\""
    );
    assert!(
        screen.contains_color(Color::Rgb(80, 170, 170)),
        "expected comment muted cyan on // line"
    );
}
