//! Unit tests: the built-in editor overlay — the unsaved-changes dialog, the
//! modified marker in the title and horizontal scrolling.
//!
//! Everything renders through the real `ui` entry point onto a `TestBackend`
//! and then asserts on the buffer cells of the *region* it means, never on a
//! flattened screen (except for the deliberate "this text is nowhere" checks).
//!
//! An 80x40 screen is used instead of the usual 80x24: the dialog is
//! `centered_rect(50, 20, …)`, so on 24 rows it is only 4 rows tall and the
//! `[Y]es [N]o [Esc]` line is clipped away by the bottom border.

use super::*;
use crate::models::EditState;

use crate::models::Editor;

use ratatui::backend::TestBackend;
use ratatui::Terminal;

use std::path::PathBuf;

use tempfile::TempDir;

const SCREEN_W: u16 = 80;
const SCREEN_H: u16 = 40;

/// Editor overlay rectangle — the same `centered_rect` call `render_overlays`
/// makes, so the tests below address real cells instead of magic numbers.
fn editor_area() -> Rect {
    centered_rect(90, 90, Rect::new(0, 0, SCREEN_W, SCREEN_H))
}

/// Unsaved-changes dialog rectangle.
fn dialog_area() -> Rect {
    centered_rect(50, 20, Rect::new(0, 0, SCREEN_W, SCREEN_H))
}

/// App whose panels both list an empty directory: a panel pointed at the
/// working directory would print real file names into the buffer and could
/// satisfy a text assertion by accident.
fn app_on_empty_dirs() -> (App, TempDir, TempDir) {
    let left = tempfile::tempdir().unwrap();
    let right = tempfile::tempdir().unwrap();
    let mut app = App::new().unwrap();
    app.left_panel = Panel::new(left.path().to_path_buf()).unwrap();
    app.right_panel = Panel::new(right.path().to_path_buf()).unwrap();
    (app, left, right)
}

/// Editor over 40 numbered lines; `BODYnn` sits at char 20 of every line, i.e.
/// inside the horizontal span the dialog covers.
fn editor_with_numbered_body(title: &str) -> Editor {
    let content = (0..40)
        .map(|i| format!("{}BODY{i:02}", " ".repeat(20)))
        .collect::<Vec<_>>()
        .join("\n");
    Editor::from_content(title.to_string(), PathBuf::from("notes.txt"), &content)
}

fn draw(app: &mut App) -> Terminal<TestBackend> {
    let mut terminal = Terminal::new(TestBackend::new(SCREEN_W, SCREEN_H)).unwrap();
    terminal.draw(|f| ui(f, app)).unwrap();
    terminal
}

/// `width` cells of row `y`, starting at column `x`.
fn cells(terminal: &Terminal<TestBackend>, y: u16, x: u16, width: u16) -> String {
    let buf = terminal.backend().buffer();
    (x..x + width).map(|col| buf[(col, y)].symbol()).collect()
}

fn row_text(terminal: &Terminal<TestBackend>, y: u16) -> String {
    cells(terminal, y, 0, SCREEN_W)
}

/// The whole screen as one string — only for "this text appears nowhere".
fn screen_text(terminal: &Terminal<TestBackend>) -> String {
    (0..SCREEN_H)
        .map(|y| row_text(terminal, y))
        .collect::<Vec<_>>()
        .join("\n")
}

/// Editor body row that shows source line `index` (no vertical scrolling while
/// the cursor sits on line 0).
fn body_row(index: u16) -> u16 {
    editor_area().y + 1 + index
}

/// Inverse of [`body_row`]: which source line a screen row shows.
fn body_row_source_index(row: u16) -> u16 {
    row - (editor_area().y + 1)
}

// -----------------------------------------------------------------------
// Unsaved-changes dialog (Esc with a modified buffer)
// -----------------------------------------------------------------------

#[test]
fn test_confirm_exit_dialog_names_the_file_being_edited() {
    let (mut app, _left, _right) = app_on_empty_dirs();
    let mut editor = editor_with_numbered_body("notes.txt");
    editor.state = EditState::ConfirmingExit;
    app.set_overlay(Overlay::Editor(editor));

    let terminal = draw(&mut app);
    let dialog = dialog_area();

    assert_eq!(
        cells(&terminal, dialog.y, dialog.x + 1, 17),
        " Unsaved Changes ",
        "dialog title row:\n{}",
        row_text(&terminal, dialog.y)
    );
    assert_eq!(
        cells(&terminal, dialog.y + 1, dialog.x + 1, 26),
        "Save changes to notes.txt?",
        "dialog prompt row:\n{}",
        row_text(&terminal, dialog.y + 1)
    );
}

#[test]
fn test_confirm_exit_dialog_offers_yes_no_and_continue_editing() {
    let (mut app, _left, _right) = app_on_empty_dirs();
    let mut editor = editor_with_numbered_body("notes.txt");
    editor.state = EditState::ConfirmingExit;
    app.set_overlay(Overlay::Editor(editor));

    let terminal = draw(&mut app);
    let dialog = dialog_area();

    // Prompt, blank spacer, choices — the choices are the third content line.
    assert_eq!(
        cells(&terminal, dialog.y + 3, dialog.x + 1, 35),
        "[Y]es  [N]o  [Esc] Continue editing",
        "dialog choices row:\n{}",
        row_text(&terminal, dialog.y + 3)
    );
}

#[test]
fn test_editor_without_confirm_exit_draws_no_dialog() {
    let (mut app, _left, _right) = app_on_empty_dirs();
    let editor = editor_with_numbered_body("notes.txt");
    assert!(!editor.state.is_confirming_exit(), "precondition");
    app.set_overlay(Overlay::Editor(editor));

    let terminal = draw(&mut app);
    let screen = screen_text(&terminal);

    for absent in [
        "Unsaved Changes",
        "Save changes to",
        "[Y]es",
        "[N]o",
        "[Esc]",
    ] {
        assert!(
            !screen.contains(absent),
            "'{absent}' must not be drawn without confirm_exit, got:\n{screen}"
        );
    }
    // The editor itself is on screen, so the check above is about the dialog
    // and not about a frame that failed to render anything at all.
    assert!(
        screen.contains("Edit: notes.txt"),
        "editor should still be open, got:\n{screen}"
    );
}

#[test]
fn test_confirm_exit_dialog_is_painted_over_the_editor_body() {
    let (mut app, _left, _right) = app_on_empty_dirs();
    let mut editor = editor_with_numbered_body("notes.txt");
    editor.state = EditState::ConfirmingExit;
    app.set_overlay(Overlay::Editor(editor));

    let terminal = draw(&mut app);
    let dialog = dialog_area();

    // The prompt row would otherwise show this source line — the marker is at
    // char 20, well inside the dialog's columns.
    let covered = body_row_source_index(dialog.y + 1);
    let covered_marker = format!("BODY{covered:02}");
    assert!(
        !row_text(&terminal, dialog.y + 1).contains(&covered_marker),
        "{covered_marker} must be hidden by the dialog, got:\n{}",
        row_text(&terminal, dialog.y + 1)
    );

    // …while the editor keeps rendering above the dialog: the dialog is drawn
    // on top of the editor, not instead of it.
    let visible = body_row_source_index(dialog.y - 1);
    let visible_marker = format!("BODY{visible:02}");
    assert!(
        row_text(&terminal, dialog.y - 1).contains(&visible_marker),
        "{visible_marker} must stay visible above the dialog, got:\n{}",
        row_text(&terminal, dialog.y - 1)
    );
}

#[test]
fn test_confirm_exit_dialog_is_centered_clear_of_the_screen_edge() {
    let (mut app, _left, _right) = app_on_empty_dirs();
    let mut editor = editor_with_numbered_body("notes.txt");
    editor.state = EditState::ConfirmingExit;
    app.set_overlay(Overlay::Editor(editor));

    let terminal = draw(&mut app);
    let dialog = dialog_area();
    let editor_rect = editor_area();

    // Find the drawn corners instead of trusting the layout: equal margins on
    // both sides is what "centered" means on screen.
    let left = (0..SCREEN_W)
        .find(|&x| cells(&terminal, dialog.y, x, 1) == "┌")
        .expect("dialog top-left corner");
    let right = (0..SCREEN_W)
        .rev()
        .find(|&x| cells(&terminal, dialog.y, x, 1) == "┐")
        .expect("dialog top-right corner");
    assert!(left > 0, "dialog must not touch column 0");
    assert_eq!(
        left,
        SCREEN_W - 1 - right,
        "margins must match, got left={left} right={right}:\n{}",
        row_text(&terminal, dialog.y)
    );

    // Left of the dialog the editor is untouched, so the dialog cannot be
    // spilling across the whole row.
    let prompt_row = dialog.y + 1;
    assert_eq!(
        cells(&terminal, prompt_row, editor_rect.x, 1),
        "│",
        "editor border left of the dialog:\n{}",
        row_text(&terminal, prompt_row)
    );
    assert!(
        !cells(&terminal, prompt_row, 0, dialog.x).contains("Save"),
        "prompt text must stay inside the dialog:\n{}",
        row_text(&terminal, prompt_row)
    );
}

// -----------------------------------------------------------------------
// Modified marker in the editor title
// -----------------------------------------------------------------------

#[test]
fn test_editor_title_marks_a_modified_buffer_with_a_star() {
    let (mut app, _left, _right) = app_on_empty_dirs();
    let mut editor = editor_with_numbered_body("notes.txt");
    editor.insert_char('x'); // the only supported way to set `modified`
    assert!(editor.state.is_modified(), "precondition");
    app.set_overlay(Overlay::Editor(editor));

    let terminal = draw(&mut app);
    let area = editor_area();

    assert_eq!(
        cells(&terminal, area.y, area.x + 1, 40),
        " Edit: notes.txt * (F2 Save, Esc Close) ",
        "editor title row:\n{}",
        row_text(&terminal, area.y)
    );
}

#[test]
fn test_editor_title_has_no_star_for_an_unmodified_buffer() {
    let (mut app, _left, _right) = app_on_empty_dirs();
    let editor = editor_with_numbered_body("notes.txt");
    assert!(!editor.state.is_modified(), "precondition");
    app.set_overlay(Overlay::Editor(editor));

    let terminal = draw(&mut app);
    let area = editor_area();

    // Two border dashes are read past the title: an appearing marker would push
    // them out, so this pins the absence of " *" and not just its prefix.
    assert_eq!(
        cells(&terminal, area.y, area.x + 1, 40),
        " Edit: notes.txt (F2 Save, Esc Close) ──",
        "editor title row:\n{}",
        row_text(&terminal, area.y)
    );
}

// -----------------------------------------------------------------------
// Horizontal scrolling: the cursor must stay inside the visible window
// -----------------------------------------------------------------------

#[test]
fn test_editor_scrolls_horizontally_so_the_cursor_column_stays_visible() {
    /// Editor inner width: the 72-column overlay less its two borders.
    const INNER_W: u16 = 70;
    const LINE_LEN: usize = 200;

    let (mut app, _left, _right) = app_on_empty_dirs();
    let line: String = "abcdefghijklmnopqrstuvwxyz"
        .chars()
        .cycle()
        .take(LINE_LEN)
        .collect();
    let mut editor = Editor::from_content("wide.txt".to_string(), PathBuf::from("wide.txt"), &line);
    editor.move_end();
    assert_eq!(editor.cursor_col, LINE_LEN, "precondition: cursor at EOL");
    app.set_overlay(Overlay::Editor(editor));

    let mut terminal = draw(&mut app);
    let area = editor_area();

    // The cursor sits one past the last char, on the rightmost inner column,
    // so the window starts LINE_LEN - (INNER_W - 1) chars into the line.
    let first_visible = LINE_LEN - (usize::from(INNER_W) - 1);
    let expected: String = line.chars().skip(first_visible).collect();
    assert_eq!(
        cells(&terminal, body_row(0), area.x + 1, INNER_W - 1),
        expected,
        "visible window must end at the cursor, got row:\n{}",
        row_text(&terminal, body_row(0))
    );
    // …and the head of the line has scrolled out of view.
    assert!(
        !expected.starts_with(&line.chars().take(3).collect::<String>()),
        "the line must not still start at the left edge"
    );

    // The terminal cursor follows the same offset instead of running off the
    // right edge of the overlay.
    assert_eq!(
        terminal.get_cursor_position().unwrap(),
        ratatui::layout::Position::new(area.x + 1 + (INNER_W - 1), body_row(0)),
        "terminal cursor position"
    );
}
