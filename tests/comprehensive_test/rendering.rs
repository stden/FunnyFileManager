//! Purpose: Rendering of console mode and the drive, delete and new-file overlays.

use super::*;

// ==================== UI Rendering: Console Mode ====================

#[test]
fn test_ui_render_console_mode() {
    let backend = ratatui::backend::TestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).unwrap();

    let mut app = App::new().unwrap();
    app.view = ViewMode::Console;

    terminal
        .draw(|f| funny_file_manager::ui::ui(f, &mut app))
        .unwrap();
}

// ==================== UI Rendering: Drive Selection Overlay ====================

#[test]
fn test_ui_render_drive_selection_left() {
    let backend = ratatui::backend::TestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).unwrap();

    let mut app = App::new().unwrap();
    app.set_overlay(Overlay::DriveSelection {
        side: PanelSide::Left,
        selected_index: 0,
        rows: funny_file_manager::models::drive_rows(),
    });

    terminal
        .draw(|f| funny_file_manager::ui::ui(f, &mut app))
        .unwrap();
}

#[test]
fn test_ui_render_drive_selection_right() {
    let backend = ratatui::backend::TestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).unwrap();

    let mut app = App::new().unwrap();
    app.set_overlay(Overlay::DriveSelection {
        side: PanelSide::Right,
        selected_index: 0,
        rows: funny_file_manager::models::drive_rows(),
    });

    terminal
        .draw(|f| funny_file_manager::ui::ui(f, &mut app))
        .unwrap();
}

// ==================== UI Rendering: Delete Confirmation Overlay ====================

#[test]
fn test_ui_render_delete_confirm_few_files() {
    let backend = ratatui::backend::TestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).unwrap();

    let mut app = App::new().unwrap();
    app.set_overlay(Overlay::DeleteConfirm(vec![
        PathBuf::from("/tmp/file1.txt"),
        PathBuf::from("/tmp/file2.txt"),
    ]));

    terminal
        .draw(|f| funny_file_manager::ui::ui(f, &mut app))
        .unwrap();
}

#[test]
fn test_ui_render_delete_confirm_many_files() {
    let backend = ratatui::backend::TestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).unwrap();

    let mut app = App::new().unwrap();
    app.set_overlay(Overlay::DeleteConfirm(
        (0..10)
            .map(|i| PathBuf::from(format!("/tmp/file{i}.txt")))
            .collect(),
    ));

    terminal
        .draw(|f| funny_file_manager::ui::ui(f, &mut app))
        .unwrap();
}

// ==================== UI Rendering: New File Input Overlay ====================

#[test]
fn test_ui_render_new_file_input() {
    let backend = ratatui::backend::TestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).unwrap();

    let mut app = App::new().unwrap();
    app.set_overlay(Overlay::NewFileInput("newfile.txt".to_owned().into()));

    terminal
        .draw(|f| funny_file_manager::ui::ui(f, &mut app))
        .unwrap();
}

#[test]
fn test_ui_render_new_file_input_empty() {
    let backend = ratatui::backend::TestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).unwrap();

    let mut app = App::new().unwrap();
    app.set_overlay(Overlay::NewFileInput(String::new().into()));

    terminal
        .draw(|f| funny_file_manager::ui::ui(f, &mut app))
        .unwrap();
}
