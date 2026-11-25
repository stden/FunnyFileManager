//! Unit tests: the whole screen through `ui` — either panel active, the help
//! and viewer overlays, minimal terminals, very long paths and every overlay.

use std::path::PathBuf;

use ratatui::backend::TestBackend;
use ratatui::Terminal;

use super::*;
use crate::models::DirEntry;
use crate::models::EntryKind;
use crate::models::FileSize;
use crate::models::Modified;
use crate::status::Status;

#[test]
fn test_ui_render_with_right_panel_active() {
    let backend = TestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).unwrap();

    let mut app = App::new().unwrap();
    app.active_panel = PanelSide::Right;

    terminal
        .draw(|f| {
            ui(f, &mut app);
        })
        .unwrap();
}

#[test]
fn test_ui_render_status_with_file_selection() {
    let backend = TestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).unwrap();

    let mut app = App::new().unwrap();

    // Add a file to the panel
    app.left_panel.entries.push(DirEntry {
        name: "test.txt".to_owned(),
        kind: EntryKind::File,
        size: FileSize::from_bytes(1024),
        modified: Modified::UNKNOWN,
        executable: false,
        ..DirEntry::default()
    });
    app.left_panel
        .select(Some(app.left_panel.entries.len() - 1));

    terminal
        .draw(|f| {
            ui(f, &mut app);
        })
        .unwrap();
}

#[test]
fn test_ui_render_status_no_selection() {
    let backend = TestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).unwrap();

    let mut app = App::new().unwrap();

    // Clear panel selection
    app.left_panel.select(None);

    terminal
        .draw(|f| {
            ui(f, &mut app);
        })
        .unwrap();
}

#[test]
fn test_ui_render_with_help_overlay() {
    let backend = TestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).unwrap();

    let mut app = App::new().unwrap();
    app.set_overlay(Overlay::Help);

    terminal
        .draw(|f| {
            ui(f, &mut app);
        })
        .unwrap();
}

#[test]
fn test_ui_render_with_viewer_overlay() {
    let backend = TestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).unwrap();

    let mut app = App::new().unwrap();
    app.set_overlay(Overlay::Viewer(Viewer::report(
        "test.txt",
        vec!["Line 1".to_owned(), "Line 2".to_owned()],
    )));

    terminal
        .draw(|f| {
            ui(f, &mut app);
        })
        .unwrap();
}

#[test]
fn test_ui_render_with_minimal_terminal_size() {
    let mut app = App::new().unwrap();
    app.left_panel.path = PathBuf::from("/test");
    app.right_panel.path = PathBuf::from("/test2");

    let backend = TestBackend::new(40, 10);
    let mut terminal = Terminal::new(backend).unwrap();

    terminal.draw(|f| ui(f, &mut app)).unwrap();
}

#[test]
fn test_ui_render_with_very_long_paths() {
    let mut app = App::new().unwrap();
    let long_path = "/very/long/path/that/exceeds/normal/display/width/in/terminal/window";
    app.left_panel.path = PathBuf::from(long_path);
    app.right_panel.path = PathBuf::from(long_path);
    app.set_status(Status::CommandOutput("x".repeat(200)));

    let backend = TestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).unwrap();

    terminal.draw(|f| ui(f, &mut app)).unwrap();
}

#[test]
fn test_render_overlays_with_all_types() {
    let mut app = App::new().unwrap();

    app.set_overlay(Overlay::Help);
    let backend = TestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal.draw(|f| render_overlays(f, &mut app)).unwrap();
    app.close_overlay();

    app.set_overlay(Overlay::Viewer(Viewer::report(
        "Test Viewer",
        vec!["Line 1".to_owned(), "Line 2".to_owned()],
    )));
    terminal.draw(|f| render_overlays(f, &mut app)).unwrap();
    app.close_overlay();

    app.set_overlay(Overlay::CommandLine("test command".into()));
    terminal.draw(|f| render_overlays(f, &mut app)).unwrap();
    app.close_overlay();
}
