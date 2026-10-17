//! Unit tests: the built-in editor draws a TAB as the spaces up to the next
//! tab stop, and the caret and the selection land where the text is drawn.
//!
//! The editor's tab size is the Options menu's "Editor settings" value, 4 by
//! default, which is what a buffer built with `from_content` carries.

use ratatui::backend::TestBackend;
use ratatui::Terminal;
use std::path::PathBuf;
use tempfile::TempDir;

use super::*;
use crate::models::{Editor, EditorMotion, Panel, SelectionMode};

const WIDTH: u16 = 40;

fn app_editing(content: &str) -> (App, TempDir) {
    let temp = TempDir::new().expect("test setup");
    let mut app = App::with_panels(
        Panel::new(temp.path().to_path_buf()).expect("test setup"),
        Panel::new(temp.path().to_path_buf()).expect("test setup"),
    );
    let editor = Editor::from_content("t.txt".to_owned(), PathBuf::from("t.txt"), content);
    app.set_overlay(Overlay::Editor(editor));
    (app, temp)
}

fn draw(app: &mut App) -> Terminal<TestBackend> {
    let mut terminal = Terminal::new(TestBackend::new(WIDTH, 8)).expect("terminal");
    terminal.draw(|f| ui(f, app)).expect("render");
    terminal
}

/// Body row `line` (row 0 is the editor's bar), trailing blanks trimmed.
fn body_row(terminal: &Terminal<TestBackend>, line: u16) -> String {
    let buffer = terminal.backend().buffer();
    let row: String = (0..WIDTH).map(|x| buffer[(x, line + 1)].symbol()).collect();
    row.trim_end().to_owned()
}

/// A TAB drew as nothing: ratatui drops control characters, and the cell
/// arithmetic counted it as zero wide, so `x\ty` read `xy` and an indented
/// line sat flush left. The F3 viewer expands the same file.
#[test]
fn tabs_are_drawn_as_spaces_to_the_next_tab_stop() {
    let (mut app, _temp) = app_editing("\tone tab\n\t\ttwo tabs\tmid\nx\ty\nabcd\te");

    let terminal = draw(&mut app);

    assert_eq!(body_row(&terminal, 0), "    one tab");
    assert_eq!(body_row(&terminal, 1), "        two tabs    mid");
    assert_eq!(body_row(&terminal, 2), "x   y");
    assert_eq!(
        body_row(&terminal, 3),
        "abcd    e",
        "a TAB on a stop is a full stop wide"
    );
}

/// Stepping over a TAB used to leave the caret where it was, so the bar's
/// column and the caret disagreed. The caret on a TAB sits at its first cell;
/// after it, at the next stop.
#[test]
fn the_caret_sits_where_the_text_after_a_tab_is_drawn() {
    let (mut app, _temp) = app_editing("x\ty");
    let editor = app.get_editor_mut().expect("editor");

    editor.cursor_col = 1; // on the TAB
    let on_tab = draw(&mut app).get_cursor_position().expect("cursor").x;
    app.get_editor_mut().expect("editor").cursor_col = 2; // on the 'y'
    let after_tab = draw(&mut app).get_cursor_position().expect("cursor").x;

    assert_eq!(on_tab, 1);
    assert_eq!(after_tab, 4);
}

/// A selected TAB is painted across every cell it draws.
#[test]
fn a_selected_tab_is_painted_across_its_whole_width() {
    let (mut app, _temp) = app_editing("x\ty");
    let editor = app.get_editor_mut().expect("editor");
    editor.cursor_col = 1;
    editor.move_cursor(EditorMotion::End, SelectionMode::Extend);

    let terminal = draw(&mut app);

    let buffer = terminal.backend().buffer();
    let painted: Vec<u16> = (0..WIDTH)
        .filter(|&x| buffer[(x, 1)].bg == palette::CURSOR_BG)
        .collect();
    assert_eq!(painted, [1, 2, 3, 4], "the TAB's three cells and the 'y'");
}
