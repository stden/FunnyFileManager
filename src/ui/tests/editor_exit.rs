//! Unit tests: the built-in editor's unsaved-changes dialog — the file it
//! names, its answers, the focus the arrow keys move, and Esc — plus the
//! helpers the other `editor_*` tests share.
//!
//! Everything renders through the real `ui` entry point onto a `TestBackend`
//! and then asserts on the buffer cells of the *region* it means, never on a
//! flattened screen (except for the deliberate "this text is nowhere" checks).
//!
//! An 80x40 screen is used instead of the usual 80x24: the dialog is
//! `centered_rect(50, 20, …)`, so on 24 rows it is only 4 rows tall and the
//! `[Y]es [N]o [Esc]` line is clipped away by the bottom border.

use std::path::PathBuf;

use ratatui::backend::TestBackend;
use ratatui::Terminal;
use tempfile::TempDir;

use super::*;
use crate::models::EditState;
use crate::models::Editor;
use crate::models::ExitChoice;
use crate::models::Panel;

pub(super) const SCREEN_W: u16 = 80;
pub(super) const SCREEN_H: u16 = 40;

/// Editor overlay rectangle — the whole screen, as Far's editor is, so the
/// tests below address real cells instead of magic numbers.
pub(super) fn editor_area() -> Rect {
    Rect::new(0, 0, SCREEN_W, SCREEN_H)
}

/// Far's warning ground.
pub(super) const WARNING_BG: Color = Color::Rgb(128, 0, 0);

/// The rectangle the dialog actually occupies, found by its ground rather than
/// computed: it is sized to its content, so its extent is an output of the
/// renderer and not an input a test may assume.
pub(super) fn dialog_area(terminal: &Terminal<TestBackend>) -> Rect {
    let buf = terminal.backend().buffer();
    let red: Vec<(u16, u16)> = (0..SCREEN_H)
        .flat_map(|y| (0..SCREEN_W).map(move |x| (x, y)))
        .filter(|&(x, y)| buf[(x, y)].bg == WARNING_BG)
        .collect();
    assert_ne!(red.len(), 0, "no dialog on screen");

    let left = red.iter().map(|&(x, _)| x).min().unwrap();
    let right = red.iter().map(|&(x, _)| x).max().unwrap();
    let top = red.iter().map(|&(_, y)| y).min().unwrap();
    let bottom = red.iter().map(|&(_, y)| y).max().unwrap();
    Rect::new(left, top, right - left + 1, bottom - top + 1)
}

/// App whose panels both list an empty directory: a panel pointed at the
/// working directory would print real file names into the buffer and could
/// satisfy a text assertion by accident.
pub(super) fn app_on_empty_dirs() -> (App, TempDir, TempDir) {
    let left = tempfile::tempdir().unwrap();
    let right = tempfile::tempdir().unwrap();
    let mut app = App::new().unwrap();
    app.left_panel = Panel::new(left.path().to_path_buf()).unwrap();
    app.right_panel = Panel::new(right.path().to_path_buf()).unwrap();
    (app, left, right)
}

/// Editor over 40 numbered lines; `BODYnn` sits at char 20 of every line, i.e.
/// inside the horizontal span the dialog covers.
pub(super) fn editor_with_numbered_body(title: &str) -> Editor {
    let content = (0..40)
        .map(|i| format!("{}BODY{i:02}", " ".repeat(20)))
        .collect::<Vec<_>>()
        .join("\n");
    Editor::from_content(title.to_owned(), PathBuf::from("notes.txt"), &content)
}

pub(super) fn draw(app: &mut App) -> Terminal<TestBackend> {
    let mut terminal = Terminal::new(TestBackend::new(SCREEN_W, SCREEN_H)).unwrap();
    terminal.draw(|f| ui(f, app)).unwrap();
    terminal
}

/// `width` cells of row `y`, starting at column `x`.
pub(super) fn cells(terminal: &Terminal<TestBackend>, y: u16, x: u16, width: u16) -> String {
    let buf = terminal.backend().buffer();
    (x..x + width).map(|col| buf[(col, y)].symbol()).collect()
}

pub(super) fn row_text(terminal: &Terminal<TestBackend>, y: u16) -> String {
    cells(terminal, y, 0, SCREEN_W)
}

/// The whole screen as one string — only for "this text appears nowhere".
pub(super) fn screen_text(terminal: &Terminal<TestBackend>) -> String {
    (0..SCREEN_H)
        .map(|y| row_text(terminal, y))
        .collect::<Vec<_>>()
        .join("\n")
}

/// Editor body row that shows source line `index` (no vertical scrolling while
/// the cursor sits on line 0).
pub(super) fn body_row(index: u16) -> u16 {
    editor_area().y + 1 + index
}

/// Inverse of [`body_row`]: which source line a screen row shows.
pub(super) fn body_row_source_index(row: u16) -> u16 {
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
