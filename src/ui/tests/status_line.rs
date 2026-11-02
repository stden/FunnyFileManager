//! Unit tests: what the active panel's info row says — the selected file,
//! extreme values, quick search before any message — and the console mode.

use ratatui::backend::TestBackend;
use ratatui::Terminal;

use super::*;
use crate::models::DirEntry;
use crate::models::EntryKind;
use crate::models::FileSize;
use crate::models::Modified;
use crate::quicksearch::QuickSearch;
use crate::status::Status;

#[test]
fn test_render_status_right_panel_active_with_file() {
    let backend = TestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).unwrap();

    let mut app = App::new().unwrap();
    app.active_panel = PanelSide::Right;

    // Add a file to the right panel
    app.right_panel.entries.push(DirEntry {
        name: "document.pdf".to_owned(),
        kind: EntryKind::File,
        size: FileSize::from_bytes(2_048_576), // 2MB,
        modified: Modified::UNKNOWN,
        executable: false,
        ..DirEntry::default()
    });
    app.right_panel
        .select(Some(app.right_panel.entries.len() - 1));

    terminal
        .draw(|f| {
            ui(f, &mut app);
        })
        .unwrap();
}

#[test]
fn test_render_status_with_extreme_values() {
    let mut app = App::new().unwrap();

    *app.left_panel.entries = vec![
        DirEntry {
            name: "..".to_owned(),
            kind: EntryKind::Directory,
            size: FileSize::from_bytes(0),
            modified: Modified::UNKNOWN,
            executable: false,
            ..DirEntry::default()
        },
        DirEntry {
            name: "huge_file.dat".to_owned(),
            kind: EntryKind::File,
            size: FileSize::from_bytes(u64::MAX),
            modified: Modified::UNKNOWN,
            executable: false,
            ..DirEntry::default()
        },
    ];
    app.left_panel.select(Some(1));

    let backend = TestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).unwrap();

    let footer = panel_footer(&app);
    terminal
        .draw(|f| {
            render_panel(f, &mut app.left_panel, f.area(), true, footer.as_ref());
        })
        .unwrap();
}

/// Quick search wins the info row: it is about what the user is typing now,
/// while the entry name only repeats the row already highlighted above it.
#[test]
fn test_panel_footer_prefers_quick_search() {
    let mut app = App::new().unwrap();
    app.quick_search = QuickSearch::Active("test".to_owned());

    assert_eq!(
        panel_footer(&app).map(|message| message.to_string()),
        Some("Search: test_".to_owned())
    );
}

/// With nothing to say, the info row falls back to the current entry's name —
/// so `panel_footer` must report "nothing", not an empty string that would
/// blank the name.
#[test]
fn test_panel_footer_is_none_when_idle() {
    let mut app = App::new().unwrap();
    app.quick_search = QuickSearch::Idle;
    app.set_status(Status::Cleared);

    assert_eq!(panel_footer(&app), None);
}

#[test]
fn test_ui_render_with_quick_search_active() {
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

// -----------------------------------------------------------------------
// Console mode rendering
// -----------------------------------------------------------------------

#[test]
fn test_ui_render_console_mode() {
    let backend = TestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).unwrap();

    let mut app = App::new().unwrap();
    app.view = ViewMode::Console;

    // Must not panic and must return early (not render panels)
    terminal
        .draw(|f| {
            ui(f, &mut app);
        })
        .unwrap();
}
