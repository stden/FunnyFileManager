//! Unit tests: overlay dialogs (split out to keep files under 500 lines).
#![allow(unused_imports)]

use super::tests::*;
use crate::models::EntryKind;
use crate::models::FileSize;
use crate::quicksearch::QuickSearch;
use crate::scroll::ScrollTop;

use super::*;

use crate::models::{App, DirEntry, Overlay};

use ratatui::backend::TestBackend;

use ratatui::Terminal;

use std::path::PathBuf;

// -----------------------------------------------------------------------
// Drive selection overlay – both panel titles
// -----------------------------------------------------------------------

#[test]
fn test_render_overlays_drive_selection_left_panel() {
    let backend = TestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).unwrap();

    let mut app = App::new().unwrap();
    app.set_overlay(Overlay::DriveSelection {
        side: crate::models::PanelSide::Left,
        selected_index: 0,
    });

    terminal.draw(|f| render_overlays(f, &app)).unwrap();
}

#[test]
fn test_render_overlays_drive_selection_right_panel() {
    let backend = TestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).unwrap();

    let mut app = App::new().unwrap();
    app.set_overlay(Overlay::DriveSelection {
        side: crate::models::PanelSide::Right,
        selected_index: 0,
    });

    terminal.draw(|f| render_overlays(f, &app)).unwrap();
}

#[test]
fn test_ui_render_drive_selection_right() {
    let backend = TestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).unwrap();

    let mut app = App::new().unwrap();
    app.set_overlay(Overlay::DriveSelection {
        side: crate::models::PanelSide::Right,
        selected_index: 0,
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

    let drives = crate::models::App::get_all_drives();
    let mut app = App::new().unwrap();
    let selected_index = if drives.is_empty() {
        0
    } else {
        drives.len() - 1
    };
    app.set_overlay(Overlay::DriveSelection {
        side: crate::models::PanelSide::Left,
        selected_index,
    });

    terminal.draw(|f| render_overlays(f, &app)).unwrap();
}

// -----------------------------------------------------------------------
// Delete confirmation overlay – >5 files branch
// -----------------------------------------------------------------------

#[test]
fn test_render_overlays_delete_confirm_many_files() {
    let backend = TestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).unwrap();

    let mut app = App::new().unwrap();
    // Create more than 5 paths to trigger the "... and N more" branch.
    let paths: Vec<std::path::PathBuf> = (0..8)
        .map(|i| std::path::PathBuf::from(format!("/tmp/file{i}.txt")))
        .collect();
    app.set_overlay(Overlay::DeleteConfirm(paths));

    terminal.draw(|f| render_overlays(f, &app)).unwrap();
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
    let paths: Vec<std::path::PathBuf> = (0..3)
        .map(|i| std::path::PathBuf::from(format!("/tmp/file{i}.txt")))
        .collect();
    app.set_overlay(Overlay::DeleteConfirm(paths));

    terminal.draw(|f| render_overlays(f, &app)).unwrap();
}

// -----------------------------------------------------------------------
// New file input overlay
// -----------------------------------------------------------------------

#[test]
fn test_render_overlays_new_file_input() {
    let backend = TestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).unwrap();

    let mut app = App::new().unwrap();
    app.set_overlay(Overlay::NewFileInput("myfile.txt".to_string()));

    terminal.draw(|f| render_overlays(f, &app)).unwrap();
}

#[test]
fn test_render_overlays_new_file_input_empty() {
    let backend = TestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).unwrap();

    let mut app = App::new().unwrap();
    app.set_overlay(Overlay::NewFileInput(String::new()));

    terminal.draw(|f| render_overlays(f, &app)).unwrap();
}

// -----------------------------------------------------------------------
// Status bar with directory selection
// -----------------------------------------------------------------------

#[test]
fn test_render_status_with_directory_selection() {
    let mut app = App::new().unwrap();
    app.left_panel.entries = vec![crate::models::DirEntry {
        name: "subdir".to_string(),
        kind: EntryKind::Directory,
        size: FileSize::from_bytes(0),
    }];
    app.left_panel.select(Some(0));

    let backend = TestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).unwrap();

    terminal
        .draw(|f| {
            let area = ratatui::layout::Rect::new(0, 0, 80, 2);
            render_status(f, &app, area);
        })
        .unwrap();
}

// -----------------------------------------------------------------------
// Panel with ".." selected (showing parent)
// -----------------------------------------------------------------------

#[test]
fn test_render_status_dotdot_selected() {
    let mut app = App::new().unwrap();
    // ".." entry produces None from get_selected_entry in status (selected_path).
    // But get_selected_entry itself returns the ".." entry.
    app.left_panel.entries = vec![crate::models::DirEntry {
        name: "..".to_string(),
        kind: EntryKind::Directory,
        size: FileSize::from_bytes(0),
    }];
    app.left_panel.select(Some(0));

    let backend = TestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).unwrap();

    terminal
        .draw(|f| {
            let area = ratatui::layout::Rect::new(0, 0, 80, 2);
            render_status(f, &app, area);
        })
        .unwrap();
}

/// Command line padding is empty when terminal is too narrow — and the typed
/// text still reaches the screen instead of being pushed off by the path.
#[test]
fn test_render_command_line_narrow_terminal() {
    let mut app = App::new().unwrap();
    app.left_panel.path = std::path::PathBuf::from("/a/very/long/path/that/is/wider/than/terminal");
    app.set_overlay(Overlay::CommandLine("ls".into()));

    // Use a very narrow terminal so prompt + text > width
    let backend = TestBackend::new(10, 10);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal.draw(|f| ui(f, &mut app)).unwrap();

    // 10 rows: header, panels, status, command line (row 8), F-key bar.
    let buf = terminal.backend().buffer();
    let row: String = (0..buf.area.width).map(|x| buf[(x, 8)].symbol()).collect();
    assert_eq!(row, "…inal> ls ", "command line row");
}

/// Drive selection with multiple items: non-selected items get default style
#[test]
fn test_render_drive_selection_multiple_items() {
    let mut app = App::new().unwrap();
    app.set_overlay(Overlay::DriveSelection {
        side: PanelSide::Left,
        selected_index: 1,
    });

    let backend = TestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal.draw(|f| ui(f, &mut app)).unwrap();
}

#[test]
fn test_render_status_with_quick_search_rebase_copy() {
    let mut app = App::new().unwrap();
    app.quick_search = QuickSearch::Active("test".to_string());

    let backend = TestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).unwrap();

    terminal
        .draw(|f| {
            let area = ratatui::layout::Rect::new(0, 0, 80, 2);
            render_status(f, &app, area);
        })
        .unwrap();
}

#[test]
fn test_render_status_without_quick_search_rebase_copy() {
    let mut app = App::new().unwrap();
    app.quick_search = QuickSearch::Idle;

    let backend = TestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).unwrap();

    terminal
        .draw(|f| {
            let area = ratatui::layout::Rect::new(0, 0, 80, 2);
            render_status(f, &app, area);
        })
        .unwrap();
}

#[test]
fn test_ui_render_with_quick_search_active_rebase_copy() {
    let backend = TestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).unwrap();

    let mut app = App::new().unwrap();
    app.quick_search = QuickSearch::Active("search".to_string());

    terminal
        .draw(|f| {
            ui(f, &mut app);
        })
        .unwrap();
}

// -----------------------------------------------------------------------
// Overlays must erase the cells they cover, not just restyle them
// -----------------------------------------------------------------------

/// App whose panels are full of loud `X` rows that would show through any
/// overlay that restyles its area without clearing the old glyphs.
fn app_with_loud_panels() -> App {
    let mut app = App::new().expect("test app");
    app.left_panel.path = PathBuf::from("/test");
    app.right_panel.path = PathBuf::from("/test2");
    for panel in [&mut app.left_panel, &mut app.right_panel] {
        panel.entries.clear();
        for i in 0..20 {
            panel.entries.push(DirEntry {
                name: format!("XXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXX_{i}"),
                kind: EntryKind::File,
                size: FileSize::from_bytes(0),
            });
        }
        panel.select(Some(0));
    }
    app
}

#[test]
fn test_editor_overlay_erases_the_panels_behind_it() {
    let mut app = app_with_loud_panels();
    app.set_overlay(Overlay::Editor(crate::models::Editor {
        title: "notes.txt".to_string(),
        lines: vec!["hi".to_string()],
        ..Default::default()
    }));

    let backend = TestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal.draw(|f| ui(f, &mut app)).unwrap();

    // The row holding the editor's short line must not carry panel glyphs
    // to its right — a Block restyles cells but does not erase their symbols.
    let buf = terminal.backend().buffer();
    let area = centered_rect(90, 90, buf.area);
    let row_y = area.y + 1; // first body row, where "hi" is drawn
    let row: String = (area.x + 1..area.x + area.width - 1)
        .map(|x| buf[(x, row_y)].symbol())
        .collect();
    assert!(row.contains("hi"), "editor line missing: {row:?}");
    assert!(
        !row.contains('X'),
        "panel glyphs bleed through the editor: {row:?}"
    );
}

#[test]
fn test_viewer_overlay_erases_the_panels_behind_it() {
    let mut app = app_with_loud_panels();
    app.set_overlay(Overlay::Viewer(crate::models::Viewer {
        title: "log".to_string(),
        lines: vec!["hi".to_string()],
        scroll: ScrollTop::TOP,
    }));

    let backend = TestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal.draw(|f| ui(f, &mut app)).unwrap();

    let buf = terminal.backend().buffer();
    let area = centered_rect(90, 90, buf.area);
    let row_y = area.y + 1;
    let row: String = (area.x + 1..area.x + area.width - 1)
        .map(|x| buf[(x, row_y)].symbol())
        .collect();
    assert!(row.contains("hi"), "viewer line missing: {row:?}");
    assert!(
        !row.contains('X'),
        "panel glyphs bleed through the viewer: {row:?}"
    );
}
