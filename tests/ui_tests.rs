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

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Whether a row carries a shell prompt.
///
/// `#` or `$` depending on the euid, so the suite passes run as either.
fn has_prompt(row: &str) -> bool {
    row.contains('#') || row.contains('$')
}

/// Far's panel ground, and the lighter blue of its cursor bar.
const PANEL_BG: Color = Color::Rgb(0, 0, 128);
const CURSOR_BG: Color = Color::Rgb(85, 150, 216);

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
        entry("..", EntryKind::Directory, 0),
        entry("docs", EntryKind::Directory, 0),
        entry("readme.md", EntryKind::File, 4096),
        entry("main.rs", EntryKind::File, 512),
    ];
    app.left_panel.select(Some(0));

    app.right_panel.entries = vec![
        entry("..", EntryKind::Directory, 0),
        entry("src", EntryKind::Directory, 0),
    ];
    app.right_panel.select(Some(0));
    app
}

// ---------------------------------------------------------------------------
// Layout structure tests
// ---------------------------------------------------------------------------

#[test]
fn test_layout_row_count() {
    // On a 24-row terminal the layout is Far's: the menu bar at the top, and
    // two rows below the panels.
    //   row 0       = menu bar, with the clock at its right end
    //   row 1       = panel top border, carrying the path
    //   row 2       = column headers (Name | Size | Date | Time)
    //   rows 3..19  = entries
    //   row 20      = panel info row (current entry, or a message)
    //   row 21      = panel bottom border, carrying the totals
    //   row 22      = command line
    //   row 23      = F-key bar
    let backend = TestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).unwrap();
    let mut app = test_app();

    terminal.draw(|f| ui(f, &mut app)).unwrap();

    // Row 0 is the menu bar, as it is in Far.
    let menu = row_text(&terminal, 0);
    assert!(
        menu.contains("Left"),
        "Row 0 is the menu bar, got: '{menu}'"
    );

    // Row 1 begins the panels, inside the double-line frame Far draws
    // everything with.
    let top_row = row_text(&terminal, 1);
    assert!(
        top_row.starts_with('╔'),
        "Row 1 should be the panel's top border, got: '{top_row}'"
    );
    assert!(!top_row.contains("Far Manager style"), "{top_row}");

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

    // Command line is at row 22 (chunks[2] on 24-row terminal):
    // panels=21, status=1, cmdline=1, fkeybar=1
    // Row 22 should contain the ">" prompt
    let cmd_row = row_text(&terminal, 22);
    assert!(
        has_prompt(&cmd_row),
        "Command line row should carry a shell prompt, got: '{cmd_row}'"
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
        cmd_row.contains("/home/user"),
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
/// it sits *on* a character rather than after the last one. The terminal cursor
/// occupies that same cell, so it cannot disappear between redraws.
#[test]
fn test_command_line_shows_cursor_when_active() {
    let backend = TestBackend::new(120, 24);
    let mut terminal = Terminal::new(backend).unwrap();
    let mut app = test_app();
    // Activate command line (like pressing ':')
    app.set_overlay(Overlay::CommandLine(String::new().into()));

    terminal.draw(|f| ui(f, &mut app)).unwrap();

    let caret = caret_column(&terminal, 22).expect("active command line caret");
    assert_eq!(
        terminal.get_cursor_position().unwrap(),
        ratatui::layout::Position::new(caret, 22),
        "Terminal cursor should occupy the painted caret cell"
    );
}

#[test]
fn test_command_line_cursor_stays_visible_when_inactive() {
    let backend = TestBackend::new(120, 24);
    let mut terminal = Terminal::new(backend).unwrap();
    let mut app = test_app();
    // command_line is None (inactive)

    terminal.draw(|f| ui(f, &mut app)).unwrap();

    let cmd_row = row_text(&terminal, 22);
    // The prompt and its cursor stay visible before the first character is
    // typed, just as they do after the command-line editor becomes active.
    assert!(
        has_prompt(&cmd_row),
        "Inactive command line should show prompt, got: '{cmd_row}'"
    );
    let caret = caret_column(&terminal, 22).expect("idle command line caret");
    assert_eq!(
        terminal.get_cursor_position().unwrap(),
        ratatui::layout::Position::new(caret, 22),
        "Terminal cursor should stay on the idle command line"
    );
}

// ---------------------------------------------------------------------------
// Top-row tests
// ---------------------------------------------------------------------------

#[test]
fn test_column_headers_replace_the_old_banner() {
    let backend = TestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).unwrap();
    let mut app = test_app();

    terminal.draw(|f| ui(f, &mut app)).unwrap();

    // The banner is gone — it only repeated the F-key bar.
    let screen = terminal.backend().to_string();
    assert!(!screen.contains("Far Manager style"), "{screen}");

    // Row 2 is Far's column header: menu bar, top border, then the header.
    let header = row_text(&terminal, 2);
    assert!(header.contains("Name"), "got: '{header}'");
    assert!(header.contains("Size"), "got: '{header}'");
    assert!(header.contains("Date"), "got: '{header}'");
    assert!(header.contains("Time"), "got: '{header}'");
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
    // Far splits a file row at its final dot and right-aligns the extension
    // against the Size column, so the two halves are no longer adjacent.
    assert!(
        all_text.contains("readme") && all_text.contains("md"),
        "Panel should show 'readme.md' split into name and extension"
    );
    assert!(
        all_text.contains("main") && all_text.contains("rs"),
        "Panel should show 'main.rs' split into name and extension"
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

    // Far names a directory instead of measuring it.
    assert!(
        all_text.contains("Folder"),
        "Directories should be labelled 'Folder'"
    );
    assert!(all_text.contains("Up"), "'..' should be labelled 'Up'");
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

    // Far prints the byte count itself while it fits the seven-cell column.
    assert!(
        all_text.contains("4096"),
        "Should show the exact byte count '4096'"
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
    assert_eq!(bg, PANEL_BG, "Panel background should be FAR_BLUE");
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
    assert!(all_text.contains("readme"));
}

// ---------------------------------------------------------------------------
// Status bar tests
// ---------------------------------------------------------------------------

#[test]
fn test_info_row_names_the_selected_entry() {
    let backend = TestBackend::new(120, 24);
    let mut terminal = Terminal::new(backend).unwrap();
    let mut app = test_app();
    app.left_panel.select(Some(1)); // select "docs"

    terminal.draw(|f| ui(f, &mut app)).unwrap();

    // The hint row is gone with the status bar; the panel's info row names the
    // current entry, as Far's does.
    let info = row_text(&terminal, 20);
    assert!(
        info.contains("docs"),
        "Info row should name the selected entry, got: '{info}'"
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

    let info = row_text(&terminal, 20);
    assert!(
        info.contains("readme.md"),
        "Info row should show the selected filename, got: '{info}'"
    );
}

#[test]
fn test_status_bar_with_quick_search() {
    let backend = TestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).unwrap();
    let mut app = test_app();
    app.quick_search = QuickSearch::Active("main".to_string());

    terminal.draw(|f| ui(f, &mut app)).unwrap();

    let info = row_text(&terminal, 20);
    assert!(
        info.contains("Search: main"),
        "Info row should show the quick search text, got: '{info}'"
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

    // In console mode the screen is a console: black, with the prompt on the
    // row after the newest transcript line -- here the very first one, since
    // nothing has been run yet.
    let prompt_row = row_text(&terminal, 0);
    assert!(
        prompt_row.starts_with('●'),
        "Console mode should show a prompt, got: '{prompt_row}'"
    );
    assert_eq!(
        row_text(&terminal, 23),
        "",
        "nothing is pinned to the bottom of a console"
    );

    // Panels should NOT appear
    let mut all_text = String::new();
    for y in 0..23 {
        all_text.push_str(&row_text(&terminal, y));
        all_text.push('\n');
    }
    assert!(
        !all_text.contains("readme.md"),
        "Console mode should hide panels"
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
    app.set_overlay(Overlay::Viewer(Viewer::report(
        "example.txt",
        vec!["Hello world".to_string(), "Second line".to_string()],
    )));

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
        scroll: ScrollTop::TOP.down(50, usize::MAX),
        ..Viewer::report("long.txt", many_lines)
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
        all_text.contains("Do you wish to delete 3 items?"),
        "Delete confirm should count the items, got:\n{all_text}"
    );
    // Braces mark the button Enter starts on; brackets mark the rest.
    assert!(
        all_text.contains("{ Delete }") && all_text.contains("[ Cancel ]"),
        "Delete confirm should offer both buttons, got:\n{all_text}"
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
        all_text.contains("… and 5 more"),
        "Should summarise the files past the first five, got:\n{all_text}"
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
        rows: funny_file_manager::models::drive_rows(),
    });

    terminal.draw(|f| ui(f, &mut app)).unwrap();

    let mut all_text = String::new();
    for y in 0..24 {
        all_text.push_str(&row_text(&terminal, y));
        all_text.push('\n');
    }

    // Far titles it "Change drive" for either panel: which one it targets is
    // already answered by which key opened it, so the dialog does not repeat it.
    assert!(
        all_text.contains("Change drive"),
        "the drive dialog should be drawn for the left panel"
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
        rows: funny_file_manager::models::drive_rows(),
    });

    terminal.draw(|f| ui(f, &mut app)).unwrap();

    let mut all_text = String::new();
    for y in 0..24 {
        all_text.push_str(&row_text(&terminal, y));
        all_text.push('\n');
    }

    assert!(
        all_text.contains("Change drive"),
        "the drive dialog should be drawn for the right panel"
    );
}

// ---------------------------------------------------------------------------
// Panel switching tests
// ---------------------------------------------------------------------------

#[test]
fn test_right_panel_info_row_names_its_own_entry() {
    let backend = TestBackend::new(120, 24);
    let mut terminal = Terminal::new(backend).unwrap();
    let mut app = test_app();
    app.active_panel = PanelSide::Right;
    app.right_panel.select(Some(1)); // select "src"

    terminal.draw(|f| ui(f, &mut app)).unwrap();

    // Each panel names its own current entry, so "which panel is active" is
    // answered by the cursor bar and the highlighted title, not by one shared row.
    let info = row_text(&terminal, 20);
    assert!(
        info.contains("src"),
        "The right panel's info row should name its selected entry, got: '{info}'"
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
        cmd.contains("/left"),
        "Command line should show left path when left active, got: '{cmd}'"
    );

    // Right active
    app.active_panel = PanelSide::Right;
    terminal.draw(|f| ui(f, &mut app)).unwrap();
    let cmd = row_text(&terminal, 22);
    assert!(
        cmd.contains("/right"),
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
    assert!(
        has_prompt(&cmd),
        "Wide terminal command line works: '{cmd}'"
    );
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

    let info = row_text(&terminal, 20);
    assert!(
        info.contains("Copy completed successfully"),
        "The active panel's info row should carry the message, got: '{info}'"
    );
}

// ---------------------------------------------------------------------------
// Far-style panel chrome
// ---------------------------------------------------------------------------

/// Far draws a cursor bar on the ACTIVE panel only: two highlighted rows would
/// give the screen two answers to "what does Enter act on".
#[test]
fn test_only_the_active_panel_draws_a_cursor_bar() {
    let backend = TestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).unwrap();
    let mut app = test_app();
    app.active_panel = PanelSide::Left;
    app.left_panel.select(Some(1));
    app.right_panel.select(Some(1));

    terminal.draw(|f| ui(f, &mut app)).unwrap();

    // Row 4 is the second entry row: menu bar, top border, column header,
    // then the entries.
    let left_bg = cell_bg(&terminal, 2, 4);
    let right_bg = cell_bg(&terminal, 42, 4);
    assert_eq!(left_bg, CURSOR_BG, "the active panel highlights its cursor");
    assert_ne!(
        right_bg, CURSOR_BG,
        "the inactive panel must not draw a second cursor bar"
    );
}

/// The panel footer reports what the panel holds and what the disk has left,
/// which Far prints into the bottom border rather than onto a row of its own.
#[test]
fn test_bottom_border_carries_the_totals() {
    let backend = TestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).unwrap();
    let mut app = test_app();

    terminal.draw(|f| ui(f, &mut app)).unwrap();

    // test_app() gives the left panel two files of 4096 and 512 bytes among
    // four entries: 4.5 K, and 2 files out of 4 rows.
    let border = row_text(&terminal, 21);
    assert!(border.contains("(2/4)"), "got: '{border}'");
    assert!(border.contains("4.5 K"), "got: '{border}'");
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
