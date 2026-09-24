//! Unit tests: the delete confirmation for few and many files, and the new
//! file name box, empty or holding a long Unicode name.

use std::path::PathBuf;

use ratatui::backend::TestBackend;
use ratatui::Terminal;

use super::overlay_mask::screen_text;
use super::*;

// -----------------------------------------------------------------------
// Delete confirmation overlay – >5 files branch
// -----------------------------------------------------------------------

#[test]
fn test_render_overlays_delete_confirm_many_files() {
    let backend = TestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).unwrap();

    let mut app = App::new().unwrap();
    // Create more than 5 paths to trigger the "... and N more" branch.
    let paths: Vec<PathBuf> = (0..8)
        .map(|i| PathBuf::from(format!("/tmp/file{i}.txt")))
        .collect();
    app.set_overlay(Overlay::DeleteConfirm(paths));

    terminal.draw(|f| render_overlays(f, &mut app)).unwrap();
}

#[test]
fn test_ui_render_delete_confirm_many_files() {
    let backend = TestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).unwrap();

    let mut app = App::new().unwrap();
    app.set_overlay(Overlay::DeleteConfirm(vec![
        PathBuf::from("f1"),
        PathBuf::from("f2"),
        PathBuf::from("f3"),
        PathBuf::from("f4"),
        PathBuf::from("f5"),
        PathBuf::from("f6"),
    ]));

    terminal
        .draw(|f| {
            ui(f, &mut app);
        })
        .unwrap();
}

#[test]
fn test_render_overlays_delete_confirm_few_files() {
    let backend = TestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).unwrap();

    let mut app = App::new().unwrap();
    // Exactly 3 paths – stays within the first 5 branch (no "... and N more").
    let paths: Vec<PathBuf> = (0..3)
        .map(|i| PathBuf::from(format!("/tmp/file{i}.txt")))
        .collect();
    app.set_overlay(Overlay::DeleteConfirm(paths));

    terminal.draw(|f| render_overlays(f, &mut app)).unwrap();
}

// -----------------------------------------------------------------------
// New file input overlay
// -----------------------------------------------------------------------

#[test]
fn test_render_overlays_new_file_input() {
    let backend = TestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).unwrap();

    let mut app = App::new().unwrap();
    app.set_overlay(Overlay::NewFileInput("myfile.txt".to_owned()));

    terminal.draw(|f| render_overlays(f, &mut app)).unwrap();
    assert!(screen_text(&terminal).contains("[ Cancel ]"));
    assert_eq!(app.dialog_mouse.buttons.len(), 2);
    let cell = terminal
        .backend()
        .buffer()
        .content
        .iter()
        .find(|cell| cell.symbol() == "y")
        .unwrap();
    assert_eq!(cell.fg, Color::Rgb(0, 0, 0));
    assert_eq!(cell.bg, Color::Rgb(0, 128, 128));
}

#[test]
fn test_render_overlays_new_file_input_empty() {
    let backend = TestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).unwrap();

    let mut app = App::new().unwrap();
    app.set_overlay(Overlay::NewFileInput(String::new()));

    terminal.draw(|f| render_overlays(f, &mut app)).unwrap();
}

#[test]
fn new_file_long_unicode_name_keeps_caret_inside_input() {
    let mut terminal = Terminal::new(TestBackend::new(30, 12)).unwrap();
    let mut app = App::new().unwrap();
    let name = format!("{}.txt", "界".repeat(40));
    app.set_overlay(Overlay::NewFileInput(name.clone()));
    terminal
        .draw(|frame| render_overlays(frame, &mut app))
        .unwrap();
    let cursor = terminal.get_cursor_position().unwrap();
    let body = app.dialog_mouse.body.unwrap();
    assert!(body.contains(cursor));
    assert!(screen_text(&terminal).contains(".txt"));
    assert!(matches!(&app.overlay, Overlay::NewFileInput(input) if *input == name));
}
