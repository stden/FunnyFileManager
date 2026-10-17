//! Unit tests: the info row under a directory or `..` selection, with and
//! without quick search, and the command line on a narrow terminal.

use std::path::PathBuf;

use ratatui::backend::TestBackend;
use ratatui::Terminal;

use super::*;
use crate::models::DirEntry;
use crate::models::EntryKind;
use crate::models::FileSize;
use crate::models::Modified;
use crate::quicksearch::QuickSearch;

// -----------------------------------------------------------------------
// Status bar with directory selection
// -----------------------------------------------------------------------

#[test]
fn test_render_status_with_directory_selection() {
    let mut app = App::new().unwrap();
    app.left_panel.entries = vec![DirEntry {
        name: "subdir".to_owned(),
        kind: EntryKind::Directory,
        size: FileSize::from_bytes(0),
        modified: Modified::UNKNOWN,
        executable: false,
        ..DirEntry::default()
    }];
    app.left_panel.select(Some(0));

    let backend = TestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).unwrap();

    let footer = panel_footer(&app);
    terminal
        .draw(|f| {
            render_panel(f, &mut app.left_panel, f.area(), true, footer.as_ref());
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
    app.left_panel.entries = vec![DirEntry {
        name: "..".to_owned(),
        kind: EntryKind::Directory,
        size: FileSize::from_bytes(0),
        modified: Modified::UNKNOWN,
        executable: false,
        ..DirEntry::default()
    }];
    app.left_panel.select(Some(0));

    let backend = TestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).unwrap();

    let footer = panel_footer(&app);
    terminal
        .draw(|f| {
            render_panel(f, &mut app.left_panel, f.area(), true, footer.as_ref());
        })
        .unwrap();
}

/// Command line padding is empty when terminal is too narrow — and the typed
/// text still reaches the screen instead of being pushed off by the path.
#[test]
fn test_render_command_line_narrow_terminal() {
    let mut app = App::new().unwrap();
    app.left_panel.path = PathBuf::from("/a/very/long/path/that/is/wider/than/terminal");
    app.set_overlay(Overlay::CommandLine("ls".into()));

    // Use a very narrow terminal so prompt + text > width
    let backend = TestBackend::new(10, 10);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal.draw(|f| ui(f, &mut app)).unwrap();

    // 10 rows: panels, status, command line (row 8), F-key bar.
    let buf = terminal.backend().buffer();
    let row: String = (0..buf.area.width).map(|x| buf[(x, 8)].symbol()).collect();
    assert!(
        row.contains("ls"),
        "the typed command survives a narrow row: '{row}'"
    );
}

#[test]
fn test_render_status_with_quick_search_rebase_copy() {
    let mut app = App::new().unwrap();
    app.quick_search = QuickSearch::Active("test".to_owned());

    let backend = TestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).unwrap();

    let footer = panel_footer(&app);
    terminal
        .draw(|f| {
            render_panel(f, &mut app.left_panel, f.area(), true, footer.as_ref());
        })
        .unwrap();
}

#[test]
fn test_render_status_without_quick_search_rebase_copy() {
    let mut app = App::new().unwrap();
    app.quick_search = QuickSearch::Idle;

    let backend = TestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).unwrap();

    let footer = panel_footer(&app);
    terminal
        .draw(|f| {
            render_panel(f, &mut app.left_panel, f.area(), true, footer.as_ref());
        })
        .unwrap();
}

#[test]
fn test_ui_render_with_quick_search_active_rebase_copy() {
    let backend = TestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).unwrap();

    let mut app = App::new().unwrap();
    app.quick_search = QuickSearch::Active("search".to_owned());

    terminal
        .draw(|f| {
            ui(f, &mut app);
        })
        .unwrap();
}
