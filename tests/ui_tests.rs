use funny_file_manager::models::EntryKind;
use funny_file_manager::models::FileName;
use funny_file_manager::models::FileSize;
use funny_file_manager::models::Modified;
use funny_file_manager::models::ViewMode;
use funny_file_manager::models::{App, DirEntry, Overlay, PanelSide, Viewer};
/// UI tests for `FunnyFileManager` — verify rendering output via `TestBackend` buffer.
///
/// These tests render the UI into a ratatui `TestBackend` and inspect the buffer
/// cells to verify that specific content appears at the expected screen positions.
use funny_file_manager::quicksearch::QuickSearch;
use funny_file_manager::scroll::ScrollTop;
use funny_file_manager::status::Status;
use funny_file_manager::ui::{centered_rect, ui};
use ratatui::backend::TestBackend;
use ratatui::layout::Rect;
use ratatui::style::Color;
use ratatui::Terminal;
use std::path::PathBuf;

#[path = "ui_tests/bars.rs"]
mod bars;
#[path = "ui_tests/command_line.rs"]
mod command_line;
#[path = "ui_tests/console_mode.rs"]
mod console_mode;
#[path = "ui_tests/geometry.rs"]
mod geometry;
#[path = "ui_tests/input_overlays.rs"]
mod input_overlays;
#[path = "ui_tests/panel_entries.rs"]
mod panel_entries;
#[path = "ui_tests/panel_focus.rs"]
mod panel_focus;
#[path = "ui_tests/view_overlays.rs"]
mod view_overlays;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Whether a row carries a shell prompt.
///
/// `#` or `$` depending on the euid, so the suite passes run as either.
fn has_prompt(row: &str) -> bool {
    row.contains('#') || row.contains('$')
}

/// Extract a row from the `TestBackend` buffer as a String (trimmed trailing spaces).
fn row_text(terminal: &Terminal<TestBackend>, y: u16) -> String {
    let buf = terminal.backend().buffer();
    let width = buf.area.width;
    let mut s = String::new();
    for x in 0..width {
        let cell = &buf[(x, y)];
        s.push_str(cell.symbol());
    }
    s.trim_end().to_owned()
}

/// Extract the background color of a cell.
fn cell_bg(terminal: &Terminal<TestBackend>, x: u16, y: u16) -> Color {
    let buf = terminal.backend().buffer();
    buf[(x, y)].bg
}

/// Create an App with predictable test entries (no filesystem dependency).
#[expect(
    clippy::unwrap_used,
    reason = "test fixture: an app that cannot start fails the test that asked for it"
)]
fn test_app() -> App {
    let mut app = App::new().unwrap();
    // Replace entries with known data so tests are deterministic
    *app.left_panel.entries = vec![
        entry("..", EntryKind::Directory, 0),
        entry("docs", EntryKind::Directory, 0),
        entry("readme.md", EntryKind::File, 4096),
        entry("main.rs", EntryKind::File, 512),
    ];
    app.left_panel.select(Some(0));

    *app.right_panel.entries = vec![
        entry("..", EntryKind::Directory, 0),
        entry("src", EntryKind::Directory, 0),
    ];
    app.right_panel.select(Some(0));
    app
}

fn entry(name: &str, kind: EntryKind, bytes: u64) -> DirEntry {
    DirEntry {
        name: name.to_owned(),
        kind,
        size: FileSize::from_bytes(bytes),
        modified: Modified::UNKNOWN,
        executable: false,
        ..DirEntry::default()
    }
}
