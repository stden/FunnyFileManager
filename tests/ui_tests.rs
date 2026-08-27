use funny_file_manager::models::EntryKind;
use funny_file_manager::models::FileName;
use funny_file_manager::models::FileSize;
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

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Extract a row from the `TestBackend` buffer as a String (trimmed trailing spaces).
fn row_text(terminal: &Terminal<TestBackend>, y: u16) -> String {
    let buf = terminal.backend().buffer();
    let width = buf.area.width;
    let mut s = String::new();
    for x in 0..width {
        let cell = &buf[(x, y)];
        s.push_str(cell.symbol());
    }
    s.trim_end().to_string()
}

/// Extract the background color of a cell.
fn cell_bg(terminal: &Terminal<TestBackend>, x: u16, y: u16) -> Color {
    let buf = terminal.backend().buffer();
    buf[(x, y)].bg
}

/// Column of the command-line caret on row `y`, found by its background.
fn caret_column(terminal: &Terminal<TestBackend>, y: u16) -> Option<u16> {
    let width = terminal.backend().buffer().area.width;
    (0..width).find(|&x| cell_bg(terminal, x, y) == Color::Yellow)
}

/// Create an App with predictable test entries (no filesystem dependency).
fn test_app() -> App {
    let mut app = App::new().unwrap();
    // Replace entries with known data so tests are deterministic
    app.left_panel.entries = vec![
        DirEntry {
            name: "..".to_string(),
            kind: EntryKind::Directory,
            size: FileSize::from_bytes(0),
        },
        DirEntry {
            name: "docs".to_string(),
            kind: EntryKind::Directory,
            size: FileSize::from_bytes(0),
        },
        DirEntry {
            name: "readme.md".to_string(),
            kind: EntryKind::File,
            size: FileSize::from_bytes(4096),
        },
        DirEntry {
            name: "main.rs".to_string(),
            kind: EntryKind::File,
            size: FileSize::from_bytes(512),
        },
    ];
    app.left_panel.select(Some(0));

    app.right_panel.entries = vec![
        DirEntry {
            name: "..".to_string(),
            kind: EntryKind::Directory,
            size: FileSize::from_bytes(0),
        },
        DirEntry {
            name: "src".to_string(),
            kind: EntryKind::Directory,
            size: FileSize::from_bytes(0),
        },
    ];
    app.right_panel.select(Some(0));
    app
}

// ---------------------------------------------------------------------------
// Layout structure tests
// ---------------------------------------------------------------------------

#[test]
fn test_layout_row_count() {
    // On a 24-row terminal the layout is:
    //   row 0       = header
    //   rows 1..20  = panels (variable)
    //   row 20      = status bar
    //   row 21      = command line  (was missing — debug RED bg)
    //   row 22      = not used (depends on exact layout split)
    //   row 23      = F-key bar
    // Exact rows depend on ratatui layout math. We verify key rows contain expected content.
    let backend = TestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).unwrap();
    let mut app = test_app();

    terminal.draw(|f| ui(f, &mut app)).unwrap();

    // Row 0: header contains "Far Manager"
    let header = row_text(&terminal, 0);
    assert!(
        header.contains("Far Manager"),
        "Header should contain 'Far Manager', got: '{header}'"
    );

    // Last row (23): F-key bar contains "Help" and "Quit"
    let fkey = row_text(&terminal, 23);
    assert!(
        fkey.contains("Help"),
        "F-key bar should contain 'Help', got: '{fkey}'"
    );
    assert!(
        fkey.contains("Quit"),
        "F-key bar should contain 'Quit', got: '{fkey}'"
    );
}

#[test]
fn test_command_line_row_exists() {
    let backend = TestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).unwrap();
    let mut app = test_app();

    terminal.draw(|f| ui(f, &mut app)).unwrap();

    // Command line is at row 22 (chunks[3] on 24-row terminal):
    // header=1, panels=19, status=1, cmdline=1, fkeybar=1
    // Row 22 should contain the ">" prompt
    let cmd_row = row_text(&terminal, 22);
    assert!(
        cmd_row.contains('>'),
        "Command line row should contain '>' prompt, got: '{cmd_row}'"
    );
}

#[test]
fn test_command_line_row_has_black_bg() {
    let backend = TestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).unwrap();
    let mut app = test_app();

    terminal.draw(|f| ui(f, &mut app)).unwrap();

    // Command line row 22 should have Black background
    let bg = cell_bg(&terminal, 0, 22);
    assert_eq!(
        bg,
        Color::Black,
        "Command line background should be Black, got: {bg:?}"
    );
}

#[test]
fn test_command_line_shows_active_path() {
    let backend = TestBackend::new(120, 24);
    let mut terminal = Terminal::new(backend).unwrap();
    let mut app = test_app();
    app.left_panel.path = PathBuf::from("/home/user");
    app.active_panel = PanelSide::Left;

    terminal.draw(|f| ui(f, &mut app)).unwrap();

    let cmd_row = row_text(&terminal, 22);
    assert!(
        cmd_row.contains("/home/user>"),
        "Command line should show active panel path, got: '{cmd_row}'"
    );
}

#[test]
fn test_command_line_shows_typed_text() {
    let backend = TestBackend::new(120, 24);
    let mut terminal = Terminal::new(backend).unwrap();
    let mut app = test_app();
    app.set_overlay(Overlay::CommandLine("ls -la".into()));

    terminal.draw(|f| ui(f, &mut app)).unwrap();

    let cmd_row = row_text(&terminal, 22);
    assert!(
        cmd_row.contains("ls -la"),
        "Command line should show typed command text, got: '{cmd_row}'"
    );
}

/// The caret is a reversed cell, not a `_` glyph — it has to stay legible when
/// it sits *on* a character rather than after the last one. So an active prompt
/// is one with a caret-coloured cell on the command row, and an inactive one
/// has none.
#[test]
fn test_command_line_shows_cursor_when_active() {
    let backend = TestBackend::new(120, 24);
    let mut terminal = Terminal::new(backend).unwrap();
    let mut app = test_app();
    // Activate command line (like pressing ':')
    app.set_overlay(Overlay::CommandLine(String::new().into()));

    terminal.draw(|f| ui(f, &mut app)).unwrap();

    assert!(
        caret_column(&terminal, 22).is_some(),
        "Active command line should show the caret, got: '{}'",
        row_text(&terminal, 22)
    );
}

#[test]
fn test_command_line_no_cursor_when_inactive() {
    let backend = TestBackend::new(120, 24);
    let mut terminal = Terminal::new(backend).unwrap();
    let mut app = test_app();
    // command_line is None (inactive)

    terminal.draw(|f| ui(f, &mut app)).unwrap();

    let cmd_row = row_text(&terminal, 22);
    // The prompt stays visible, but there is no caret to draw.
    assert!(
        cmd_row.contains('>'),
        "Inactive command line should show prompt, got: '{cmd_row}'"
    );
    assert_eq!(
        caret_column(&terminal, 22),
        None,
        "Inactive command line should show no caret, got: '{cmd_row}'"
    );
}

// ---------------------------------------------------------------------------
// Header tests
// ---------------------------------------------------------------------------

#[test]
fn test_header_content() {
    let backend = TestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).unwrap();
    let mut app = test_app();

    terminal.draw(|f| ui(f, &mut app)).unwrap();

    let header = row_text(&terminal, 0);
    assert!(header.contains("Tab: Switch panels"));
    assert!(header.contains("F1 for help"));
}

// ---------------------------------------------------------------------------
// Panel rendering tests
// ---------------------------------------------------------------------------

#[test]
fn test_panels_show_directory_entries() {
    let backend = TestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).unwrap();
    let mut app = test_app();

    terminal.draw(|f| ui(f, &mut app)).unwrap();

    // Collect all rendered text to check panel entries appear somewhere
    let mut all_text = String::new();
    for y in 0..24 {
        all_text.push_str(&row_text(&terminal, y));
        all_text.push('\n');
    }

    assert!(all_text.contains("docs"), "Panel should show 'docs' entry");
    assert!(
        all_text.contains("readme.md"),
        "Panel should show 'readme.md' entry"
    );
    assert!(
        all_text.contains("main.rs"),
        "Panel should show 'main.rs' entry"
    );
    assert!(all_text.contains("src"), "Right panel should show 'src'");
}

#[test]
fn test_panels_show_dir_marker() {
    // Default 80-column terminal: the size column must fit there, not only on a
    // wide one — a fixed 30-cell name field used to leave 3 columns for it.
    let backend = TestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).unwrap();
    let mut app = test_app();

    terminal.draw(|f| ui(f, &mut app)).unwrap();

    let mut all_text = String::new();
    for y in 0..24 {
        all_text.push_str(&row_text(&terminal, y));
        all_text.push('\n');
    }

    // Directories show <DIR>
    assert!(
        all_text.contains("<DIR>"),
        "Directories should show <DIR> marker"
    );
}

#[test]
fn test_panel_shows_file_sizes() {
    // Default 80-column terminal — see test_panels_show_dir_marker.
    let backend = TestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).unwrap();
    let mut app = test_app();

    terminal.draw(|f| ui(f, &mut app)).unwrap();

    let mut all_text = String::new();
    for y in 0..24 {
        all_text.push_str(&row_text(&terminal, y));
        all_text.push('\n');
    }

    // 4096 bytes = 4.00 KB
    assert!(
        all_text.contains("4.00 KB"),
        "Should show formatted file size '4.00 KB'"
    );
}

#[test]
fn test_active_panel_left_by_default() {
    let backend = TestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).unwrap();
    let mut app = test_app();
    app.active_panel = PanelSide::Left;

    terminal.draw(|f| ui(f, &mut app)).unwrap();

    // Active panel border is Cyan; verify a cell on the left panel border
    // Row 1, col 0 is the top-left corner of the left panel
    let bg = cell_bg(&terminal, 0, 1);
    // The panel has FAR_BLUE background
    assert_eq!(
        bg,
        Color::Rgb(0, 0, 128),
        "Panel background should be FAR_BLUE"
    );
}

#[test]
fn test_marked_files_appear() {
    let backend = TestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).unwrap();
    let mut app = test_app();
    app.left_panel.marked.insert(FileName::from("readme.md"));

    terminal.draw(|f| ui(f, &mut app)).unwrap();

    // The marked file should still be visible
    let mut all_text = String::new();
    for y in 0..24 {
        all_text.push_str(&row_text(&terminal, y));
        all_text.push('\n');
    }
    assert!(all_text.contains("readme.md"));
}

// ---------------------------------------------------------------------------
// Status bar tests
// ---------------------------------------------------------------------------

#[test]
fn test_status_bar_shows_navigation_hints() {
    let backend = TestBackend::new(120, 24);
    let mut terminal = Terminal::new(backend).unwrap();
    let mut app = test_app();
    app.left_panel.select(Some(1)); // select "docs"

    terminal.draw(|f| ui(f, &mut app)).unwrap();

    // Status bar is at row 21 on 24-row terminal
    let status = row_text(&terminal, 21);
    assert!(
        status.contains("Navigate") || status.contains("docs"),
        "Status bar should show navigation hints or selected entry info, got: '{status}'"
    );
}

#[test]
fn test_status_bar_shows_selected_file_info() {
    let backend = TestBackend::new(120, 24);
    let mut terminal = Terminal::new(backend).unwrap();
    let mut app = test_app();
    // Select readme.md (index 2)
    app.left_panel.select(Some(2));

    terminal.draw(|f| ui(f, &mut app)).unwrap();

    let status = row_text(&terminal, 21);
    assert!(
        status.contains("readme.md"),
        "Status bar should show selected filename, got: '{status}'"
    );
}

#[test]
fn test_status_bar_with_quick_search() {
    let backend = TestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).unwrap();
    let mut app = test_app();
    app.quick_search = QuickSearch::Active("main".to_string());

    terminal.draw(|f| ui(f, &mut app)).unwrap();

    let status = row_text(&terminal, 21);
    assert!(
        status.contains("Quick search: main"),
        "Status bar should show quick search text, got: '{status}'"
    );
}

// ---------------------------------------------------------------------------
// F-key bar tests
// ---------------------------------------------------------------------------

#[test]
fn test_fkey_bar_shows_all_keys() {
    let backend = TestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).unwrap();
    let mut app = test_app();

    terminal.draw(|f| ui(f, &mut app)).unwrap();

    let fkey = row_text(&terminal, 23);
    for label in &["Help", "View", "Edit", "Copy", "Delete", "Quit"] {
        assert!(
            fkey.contains(label),
            "F-key bar should contain '{label}', got: '{fkey}'"
        );
    }
}

#[test]
fn test_fkey_bar_shows_key_numbers() {
    let backend = TestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).unwrap();
    let mut app = test_app();

    terminal.draw(|f| ui(f, &mut app)).unwrap();

    let fkey = row_text(&terminal, 23);
    assert!(fkey.contains('1'), "F-key bar should show key number 1");
    assert!(fkey.contains("10"), "F-key bar should show key number 10");
}

// ---------------------------------------------------------------------------
// Console mode tests
// ---------------------------------------------------------------------------

#[test]
fn test_console_mode_hides_panels() {
    let backend = TestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).unwrap();
    let mut app = test_app();
    app.view = ViewMode::Console;

    terminal.draw(|f| ui(f, &mut app)).unwrap();

    // In console mode, the screen should be mostly black
    // and show "Press Ctrl+O" hint at the bottom
    let hint_row = row_text(&terminal, 23);
    assert!(
        hint_row.contains("Ctrl+O"),
        "Console mode should show Ctrl+O hint, got: '{hint_row}'"
    );

    // Panels should NOT appear
    let mut all_text = String::new();
    for y in 0..23 {
        all_text.push_str(&row_text(&terminal, y));
        all_text.push('\n');
    }
    assert!(
        !all_text.contains("Far Manager"),
        "Console mode should hide the header"
    );
}

#[test]
fn test_console_mode_black_background() {
    let backend = TestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).unwrap();
    let mut app = test_app();
    app.view = ViewMode::Console;

    terminal.draw(|f| ui(f, &mut app)).unwrap();

    // Middle of the screen should have black background
    let bg = cell_bg(&terminal, 10, 10);
    assert_eq!(bg, Color::Black, "Console mode background should be black");
}

// ---------------------------------------------------------------------------
// Help overlay tests
// ---------------------------------------------------------------------------

#[test]
fn test_help_overlay_content() {
    let backend = TestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).unwrap();
    let mut app = test_app();
    app.set_overlay(Overlay::Help);

    terminal.draw(|f| ui(f, &mut app)).unwrap();

    let mut all_text = String::new();
    for y in 0..24 {
        all_text.push_str(&row_text(&terminal, y));
        all_text.push('\n');
    }

    assert!(
        all_text.contains("Help"),
        "Help overlay should show 'Help' title"
    );
    assert!(
        all_text.contains("F3: View"),
        "Help overlay should mention F3"
    );
    assert!(
        all_text.contains("F5: Copy"),
        "Help overlay should mention F5"
    );
}

// ---------------------------------------------------------------------------
// Viewer overlay tests
// ---------------------------------------------------------------------------

#[test]
fn test_viewer_overlay_shows_filename() {
    let backend = TestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).unwrap();
    let mut app = test_app();
    app.set_overlay(Overlay::Viewer(Viewer {
        title: "example.txt".to_string(),
        lines: vec!["Hello world".to_string(), "Second line".to_string()],
        scroll: ScrollTop::TOP,
    }));

    terminal.draw(|f| ui(f, &mut app)).unwrap();

    let mut all_text = String::new();
    for y in 0..24 {
        all_text.push_str(&row_text(&terminal, y));
        all_text.push('\n');
    }

    assert!(
        all_text.contains("example.txt"),
        "Viewer should show filename in title"
    );
    assert!(
        all_text.contains("Hello world"),
        "Viewer should show file content"
    );
}

#[test]
fn test_viewer_overlay_with_scroll() {
    let backend = TestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).unwrap();
    let mut app = test_app();
    let many_lines: Vec<String> = (0..100).map(|i| format!("Line {i}")).collect();
    app.set_overlay(Overlay::Viewer(Viewer {
        title: "long.txt".to_string(),
        lines: many_lines,
        scroll: ScrollTop::TOP.down(50, usize::MAX),
    }));

    terminal.draw(|f| ui(f, &mut app)).unwrap();

    let mut all_text = String::new();
    for y in 0..24 {
        all_text.push_str(&row_text(&terminal, y));
        all_text.push('\n');
    }

    // After scrolling to 50, "Line 0" should NOT be visible
    assert!(
        !all_text.contains("Line 0 "),
        "Scrolled viewer should not show Line 0"
    );
    // But "Line 50" should be visible
    assert!(
        all_text.contains("Line 50"),
        "Scrolled viewer should show Line 50"
    );
}

// ---------------------------------------------------------------------------
// Delete confirmation overlay tests
// ---------------------------------------------------------------------------

#[test]
fn test_delete_confirm_shows_file_count() {
    let backend = TestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).unwrap();
    let mut app = test_app();
    app.set_overlay(Overlay::DeleteConfirm(vec![
        PathBuf::from("/tmp/a.txt"),
        PathBuf::from("/tmp/b.txt"),
        PathBuf::from("/tmp/c.txt"),
    ]));

    terminal.draw(|f| ui(f, &mut app)).unwrap();

    let mut all_text = String::new();
    for y in 0..24 {
        all_text.push_str(&row_text(&terminal, y));
        all_text.push('\n');
    }

    assert!(
        all_text.contains("3 item(s)"),
        "Delete confirm should show item count"
    );
    assert!(
        all_text.contains("[Y]es"),
        "Delete confirm should show [Y]es option"
    );
}

#[test]
fn test_delete_confirm_truncates_long_list() {
    let backend = TestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).unwrap();
    let mut app = test_app();
    let paths: Vec<PathBuf> = (0..10)
        .map(|i| PathBuf::from(format!("file{i}.txt")))
        .collect();
    app.set_overlay(Overlay::DeleteConfirm(paths));

    terminal.draw(|f| ui(f, &mut app)).unwrap();

    let mut all_text = String::new();
    for y in 0..24 {
        all_text.push_str(&row_text(&terminal, y));
        all_text.push('\n');
    }

    assert!(
        all_text.contains("... and 5 more"),
        "Should show '... and 5 more' for >5 files"
    );
}

// ---------------------------------------------------------------------------
// New file input overlay tests
// ---------------------------------------------------------------------------

#[test]
fn test_new_file_input_overlay() {
    let backend = TestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).unwrap();
    let mut app = test_app();
    app.set_overlay(Overlay::NewFileInput("newfile.txt".to_string()));

    terminal.draw(|f| ui(f, &mut app)).unwrap();

    let mut all_text = String::new();
    for y in 0..24 {
        all_text.push_str(&row_text(&terminal, y));
        all_text.push('\n');
    }

    assert!(
        all_text.contains("Create New File"),
        "New file overlay should show title"
    );
    assert!(
        all_text.contains("newfile.txt"),
        "New file overlay should show typed filename"
    );
}

#[test]
fn test_new_file_input_empty() {
    let backend = TestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).unwrap();
    let mut app = test_app();
    app.set_overlay(Overlay::NewFileInput(String::new()));

    terminal.draw(|f| ui(f, &mut app)).unwrap();

    let mut all_text = String::new();
    for y in 0..24 {
        all_text.push_str(&row_text(&terminal, y));
        all_text.push('\n');
    }

    assert!(
        all_text.contains("Enter filename"),
        "Empty new file input should show prompt"
    );
}

// ---------------------------------------------------------------------------
// Drive selection overlay tests
// ---------------------------------------------------------------------------

#[test]
fn test_drive_selection_overlay_left() {
    let backend = TestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).unwrap();
    let mut app = test_app();
    app.set_overlay(Overlay::DriveSelection {
        side: PanelSide::Left,
        selected_index: 0,
    });

    terminal.draw(|f| ui(f, &mut app)).unwrap();

    let mut all_text = String::new();
    for y in 0..24 {
        all_text.push_str(&row_text(&terminal, y));
        all_text.push('\n');
    }

    assert!(
        all_text.contains("Left Panel"),
        "Drive selection should show 'Left Panel' in title"
    );
}

#[test]
fn test_drive_selection_overlay_right() {
    let backend = TestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).unwrap();
    let mut app = test_app();
    app.set_overlay(Overlay::DriveSelection {
        side: PanelSide::Right,
        selected_index: 0,
    });

    terminal.draw(|f| ui(f, &mut app)).unwrap();

    let mut all_text = String::new();
    for y in 0..24 {
        all_text.push_str(&row_text(&terminal, y));
        all_text.push('\n');
    }

    assert!(
        all_text.contains("Right Panel"),
        "Drive selection should show 'Right Panel' in title"
    );
}

// ---------------------------------------------------------------------------
// Panel switching tests
// ---------------------------------------------------------------------------

#[test]
fn test_right_panel_active_changes_status() {
    let backend = TestBackend::new(120, 24);
    let mut terminal = Terminal::new(backend).unwrap();
    let mut app = test_app();
    app.active_panel = PanelSide::Right;
    app.right_panel.select(Some(1)); // select "src"

    terminal.draw(|f| ui(f, &mut app)).unwrap();

    let status = row_text(&terminal, 21);
    assert!(
        status.contains("src"),
        "Status bar should show right panel's selected entry when right is active, got: '{status}'"
    );
}

#[test]
fn test_command_line_path_follows_active_panel() {
    let backend = TestBackend::new(120, 24);
    let mut terminal = Terminal::new(backend).unwrap();
    let mut app = test_app();
    app.left_panel.path = PathBuf::from("/left");
    app.right_panel.path = PathBuf::from("/right");

    // Left active
    app.active_panel = PanelSide::Left;
    terminal.draw(|f| ui(f, &mut app)).unwrap();
    let cmd = row_text(&terminal, 22);
    assert!(
        cmd.contains("/left>"),
        "Command line should show left path when left active, got: '{cmd}'"
    );

    // Right active
    app.active_panel = PanelSide::Right;
    terminal.draw(|f| ui(f, &mut app)).unwrap();
    let cmd = row_text(&terminal, 22);
    assert!(
        cmd.contains("/right>"),
        "Command line should show right path when right active, got: '{cmd}'"
    );
}

// ---------------------------------------------------------------------------
// Centered rect utility tests
// ---------------------------------------------------------------------------

#[test]
fn test_centered_rect_is_centered() {
    let base = Rect::new(0, 0, 100, 100);
    let rect = centered_rect(50, 50, base);

    // Should be roughly centered
    assert!(
        rect.x >= 20 && rect.x <= 30,
        "x should be ~25, got {}",
        rect.x
    );
    assert!(
        rect.y >= 20 && rect.y <= 30,
        "y should be ~25, got {}",
        rect.y
    );
    assert_eq!(rect.width, 50);
    assert_eq!(rect.height, 50);
}

#[test]
fn test_centered_rect_clamps_percent_x() {
    let base = Rect::new(0, 0, 100, 100);
    // percent_x > 100 should be clamped
    let rect = centered_rect(150, 50, base);
    assert_eq!(rect.width, 100);
}

// ---------------------------------------------------------------------------
// Minimal / edge-case terminal sizes
// ---------------------------------------------------------------------------

#[test]
fn test_tiny_terminal_does_not_panic() {
    let backend = TestBackend::new(20, 5);
    let mut terminal = Terminal::new(backend).unwrap();
    let mut app = test_app();

    // Should not panic even on very small terminal
    terminal.draw(|f| ui(f, &mut app)).unwrap();
}

#[test]
fn test_wide_terminal() {
    let backend = TestBackend::new(200, 24);
    let mut terminal = Terminal::new(backend).unwrap();
    let mut app = test_app();

    terminal.draw(|f| ui(f, &mut app)).unwrap();

    // Command line should fill the width with padding
    let cmd = row_text(&terminal, 22);
    assert!(cmd.contains('>'), "Wide terminal command line works");
}

// ---------------------------------------------------------------------------
// Status message display
// ---------------------------------------------------------------------------

#[test]
fn test_status_message_displayed() {
    let backend = TestBackend::new(120, 24);
    let mut terminal = Terminal::new(backend).unwrap();
    let mut app = test_app();
    app.set_status(Status::CommandOutput(
        "Copy completed successfully".to_string(),
    ));
    app.left_panel.select(Some(1));

    terminal.draw(|f| ui(f, &mut app)).unwrap();

    let status = row_text(&terminal, 21);
    assert!(
        status.contains("Copy completed successfully"),
        "Status bar should show status_message, got: '{status}'"
    );
}
