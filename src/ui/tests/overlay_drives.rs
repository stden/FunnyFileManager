//! Unit tests: the drive selection overlay over either panel, its highlighted
//! entry, and many drives, drawn through `render_overlays` and `ui`.

use ratatui::backend::TestBackend;
use ratatui::Terminal;

use super::*;

// -----------------------------------------------------------------------
// Drive selection overlay – both panel titles
// -----------------------------------------------------------------------

#[test]
fn test_render_overlays_drive_selection_left_panel() {
    let backend = TestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).unwrap();

    let mut app = App::new().unwrap();
    app.set_overlay(Overlay::DriveSelection {
        side: PanelSide::Left,
        selected_index: 0,
        rows: crate::models::drive_rows(),
    });

    terminal.draw(|f| render_overlays(f, &mut app)).unwrap();
}

#[test]
fn test_render_overlays_drive_selection_right_panel() {
    let backend = TestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).unwrap();

    let mut app = App::new().unwrap();
    app.set_overlay(Overlay::DriveSelection {
        side: PanelSide::Right,
        selected_index: 0,
        rows: crate::models::drive_rows(),
    });

    terminal.draw(|f| render_overlays(f, &mut app)).unwrap();
}

#[test]
fn test_ui_render_drive_selection_right() {
    let backend = TestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).unwrap();

    let mut app = App::new().unwrap();
    app.set_overlay(Overlay::DriveSelection {
        side: PanelSide::Right,
        selected_index: 0,
        rows: crate::models::drive_rows(),
    });

    terminal
        .draw(|f| {
            ui(f, &mut app);
        })
        .unwrap();
}

#[test]
fn test_render_overlays_drive_selection_highlighted_entry() {
    let backend = TestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).unwrap();

    let drives = App::get_all_drives();
    let mut app = App::new().unwrap();
    let selected_index = if drives.is_empty() {
        0
    } else {
        drives.len() - 1
    };
    app.set_overlay(Overlay::DriveSelection {
        side: PanelSide::Left,
        selected_index,
        rows: crate::models::drive_rows(),
    });

    terminal.draw(|f| render_overlays(f, &mut app)).unwrap();
}

/// Drive selection with multiple items: non-selected items get default style
#[test]
fn test_render_drive_selection_multiple_items() {
    let mut app = App::new().unwrap();
    app.set_overlay(Overlay::DriveSelection {
        side: PanelSide::Left,
        selected_index: 1,
        rows: crate::models::drive_rows(),
    });

    let backend = TestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal.draw(|f| ui(f, &mut app)).unwrap();
}
