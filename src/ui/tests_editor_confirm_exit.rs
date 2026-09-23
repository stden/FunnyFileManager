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
use crate::models::ExitChoice;
use crate::models::Panel;

use crate::models::Editor;

use ratatui::backend::TestBackend;
use ratatui::Terminal;

use std::path::PathBuf;

use tempfile::TempDir;

const SCREEN_W: u16 = 80;
const SCREEN_H: u16 = 40;

/// Editor overlay rectangle — the whole screen, as Far's editor is, so the
/// tests below address real cells instead of magic numbers.
fn editor_area() -> Rect {
    Rect::new(0, 0, SCREEN_W, SCREEN_H)
}

/// Far's warning ground.
const WARNING_BG: Color = Color::Rgb(128, 0, 0);

/// The rectangle the dialog actually occupies, found by its ground rather than
/// computed: it is sized to its content, so its extent is an output of the
/// renderer and not an input a test may assume.
fn dialog_area(terminal: &Terminal<TestBackend>) -> Rect {
    let buf = terminal.backend().buffer();
    let red: Vec<(u16, u16)> = (0..SCREEN_H)
        .flat_map(|y| (0..SCREEN_W).map(move |x| (x, y)))
        .filter(|&(x, y)| buf[(x, y)].bg == WARNING_BG)
        .collect();
    assert!(!red.is_empty(), "no dialog on screen");

    let left = red.iter().map(|&(x, _)| x).min().unwrap();
    let right = red.iter().map(|&(x, _)| x).max().unwrap();
    let top = red.iter().map(|&(_, y)| y).min().unwrap();
    let bottom = red.iter().map(|&(_, y)| y).max().unwrap();
    Rect::new(left, top, right - left + 1, bottom - top + 1)
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
    editor.state = EditState::ConfirmingExit(ExitChoice::default());
    app.set_overlay(Overlay::Editor(editor));

    let screen = screen_text(&draw(&mut app));

    assert!(screen.contains("Editor"), "dialog title:\n{screen}");
    assert!(
        screen.contains("File has been modified. Save?"),
        "dialog question:\n{screen}"
    );
}

#[test]
fn test_confirm_exit_dialog_offers_yes_no_and_continue_editing() {
    let (mut app, _left, _right) = app_on_empty_dirs();
    let mut editor = editor_with_numbered_body("notes.txt");
    editor.state = EditState::ConfirmingExit(ExitChoice::default());
    app.set_overlay(Overlay::Editor(editor));

    let screen = screen_text(&draw(&mut app));

    // Braces mark the button Enter starts on; brackets mark the rest.
    assert!(screen.contains("{ Yes }"), "default button:\n{screen}");
    assert!(screen.contains("[ No ]"), "discard button:\n{screen}");
    assert!(screen.contains("[ Cancel ]"), "cancel button:\n{screen}");
}

/// Right and Left walk the cursor between the buttons, so Enter is never the
/// only way to answer and never answers something the user cannot see.
#[test]
fn test_confirm_exit_focus_moves_with_the_arrow_keys() {
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

    let (mut app, _left, _right) = app_on_empty_dirs();
    let mut editor = editor_with_numbered_body("notes.txt");
    editor.state = EditState::ConfirmingExit(ExitChoice::default());
    app.set_overlay(Overlay::Editor(editor));

    let press = |app: &mut App, code| {
        crate::popup::process_popup_input(app, KeyEvent::new(code, KeyModifiers::empty()))
    };

    let focus = |app: &App| {
        app.get_editor()
            .and_then(|editor| editor.state.exit_choice())
            .expect("dialog still open")
    };

    assert_eq!(focus(&app), ExitChoice::Save, "opens on the default");
    press(&mut app, KeyCode::Right);
    assert_eq!(focus(&app), ExitChoice::Discard);
    press(&mut app, KeyCode::Right);
    assert_eq!(focus(&app), ExitChoice::Cancel);
    // …and wraps, so the cursor cannot get stuck at either end.
    press(&mut app, KeyCode::Right);
    assert_eq!(focus(&app), ExitChoice::Save);
    press(&mut app, KeyCode::Left);
    assert_eq!(focus(&app), ExitChoice::Cancel);
}

/// Esc is Cancel: it puts the user back in the editor with the buffer intact,
/// which is the one answer that can never lose work.
#[test]
fn test_confirm_exit_escape_returns_to_the_editor() {
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

    let (mut app, _left, _right) = app_on_empty_dirs();
    let mut editor = editor_with_numbered_body("notes.txt");
    editor.insert_char('x');
    editor.state = EditState::ConfirmingExit(ExitChoice::default());
    app.set_overlay(Overlay::Editor(editor));

    crate::popup::process_popup_input(&mut app, KeyEvent::new(KeyCode::Esc, KeyModifiers::empty()));

    let editor = app.get_editor().expect("editor must stay open");
    assert!(!editor.state.is_confirming_exit(), "dialog closed");
    assert!(editor.state.is_modified(), "the edit is still unsaved");
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
        screen.contains("notes.txt") && screen.contains("Ln 1/40"),
        "editor should still be open, got:\n{screen}"
    );
}

#[test]
fn test_confirm_exit_dialog_is_painted_over_the_editor_body() {
    let (mut app, _left, _right) = app_on_empty_dirs();
    let mut editor = editor_with_numbered_body("notes.txt");
    editor.state = EditState::ConfirmingExit(ExitChoice::default());
    app.set_overlay(Overlay::Editor(editor));

    let terminal = draw(&mut app);
    let dialog = dialog_area(&terminal);

    // A row the dialog covers must no longer show the editor line underneath.
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
    editor.state = EditState::ConfirmingExit(ExitChoice::default());
    app.set_overlay(Overlay::Editor(editor));

    let terminal = draw(&mut app);
    let dialog = dialog_area(&terminal);

    // Equal margins on both sides is what "centred" means on screen, and the
    // dialog is sized to its content, so it cannot fill the row either.
    assert!(dialog.x > 0, "dialog must not touch column 0");
    // Within a column: an odd leftover cannot be split evenly, and rounding it
    // one way is centred while dumping it all on one side is not.
    let right_margin = SCREEN_W - (dialog.x + dialog.width);
    assert!(
        dialog.x.abs_diff(right_margin) <= 1,
        "margins must match, got left={} right={right_margin}:\n{}",
        dialog.x,
        row_text(&terminal, dialog.y)
    );
    assert!(
        dialog.width < SCREEN_W,
        "a content-sized dialog must not span the screen"
    );

    // Left of the dialog the editor is untouched, so it cannot be spilling
    // across the whole row.
    assert!(
        !cells(&terminal, dialog.y + 1, 0, dialog.x).contains("modified"),
        "prompt text must stay inside the dialog:\n{}",
        row_text(&terminal, dialog.y + 1)
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

    // Far's editor bar: the file on the left, the cursor position on the right.
    let bar = row_text(&terminal, area.y);
    assert!(bar.starts_with(" notes.txt *"), "editor bar:\n{bar}");
    assert!(bar.trim_end().ends_with("Ch 0x0020"), "editor bar:\n{bar}");
}

#[test]
fn test_editor_title_has_no_star_for_an_unmodified_buffer() {
    let (mut app, _left, _right) = app_on_empty_dirs();
    let editor = editor_with_numbered_body("notes.txt");
    assert!(!editor.state.is_modified(), "precondition");
    app.set_overlay(Overlay::Editor(editor));

    let terminal = draw(&mut app);
    let area = editor_area();

    // A blank is read past the name: an appearing marker would fill it, so this
    // pins the absence of " *" and not just its prefix.
    let bar = row_text(&terminal, area.y);
    assert!(bar.starts_with(" notes.txt  "), "editor bar:\n{bar}");
}

// -----------------------------------------------------------------------
// Horizontal scrolling: the cursor must stay inside the visible window
// -----------------------------------------------------------------------

#[test]
fn test_editor_scrolls_horizontally_so_the_cursor_column_stays_visible() {
    /// Editor inner width: the full screen, since the editor draws no border.
    const INNER_W: u16 = SCREEN_W;
    const LINE_LEN: usize = 200;

    let (mut app, _left, _right) = app_on_empty_dirs();
    let line = alphabet_line(LINE_LEN);
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
        cells(&terminal, body_row(0), area.x, INNER_W - 1),
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
        ratatui::layout::Position::new(area.x + (INNER_W - 1), body_row(0)),
        "terminal cursor position"
    );
}

fn alphabet_line(length: usize) -> String {
    "abcdefghijklmnopqrstuvwxyz"
        .chars()
        .cycle()
        .take(length)
        .collect()
}
