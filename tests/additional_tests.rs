use funny_file_manager::models::FileName;
use funny_file_manager::models::FileSize;
/// Additional tests to improve coverage for `FunnyFileManager`.
///
/// Covers: marked file cleanup after reload, `DriveEntry` display, Panel edge cases,
/// handler error paths, command execution, and UI rendering edge cases.
use funny_file_manager::models::ViewMode;
use funny_file_manager::models::{App, DriveEntry, Overlay, Panel, PanelSide, SortMode, Viewer};
use funny_file_manager::scroll::ScrollTop;
use funny_file_manager::ui::{centered_rect, ui};
use funny_file_manager::utils::{copy_dir_all, unique_destination};
use ratatui::backend::TestBackend;
use ratatui::style::Color;
use ratatui::Terminal;
use std::fs;
use std::path::PathBuf;
use tempfile::TempDir;

// ---------------------------------------------------------------------------
// DriveEntry::display
// ---------------------------------------------------------------------------

#[test]
fn test_drive_entry_display_local() {
    let drive = DriveEntry::Local(PathBuf::from("/"));
    assert_eq!(drive.display(), "/");

    let drive2 = DriveEntry::Local(PathBuf::from("/home/user"));
    assert_eq!(drive2.display(), "/home/user");
}

#[test]
fn test_drive_entry_display_s3() {
    let drive = DriveEntry::S3Profile("production".to_string());
    assert_eq!(drive.display(), "S3: production");

    let drive2 = DriveEntry::S3Profile("default".to_string());
    assert_eq!(drive2.display(), "S3: default");
}

// ---------------------------------------------------------------------------
// Marked files cleanup after reload
// ---------------------------------------------------------------------------

#[test]
fn test_marked_files_cleaned_on_reload() {
    let temp = TempDir::new().unwrap();
    let file_a = temp.path().join("a.txt");
    let file_b = temp.path().join("b.txt");
    fs::write(&file_a, "aaa").unwrap();
    fs::write(&file_b, "bbb").unwrap();

    let mut panel = Panel::new(temp.path().to_path_buf()).unwrap();
    panel.marked.insert(FileName::from("a.txt"));
    panel.marked.insert(FileName::from("b.txt"));
    assert_eq!(panel.marked.len(), 2);

    // Delete a.txt, reload — marked should drop "a.txt"
    fs::remove_file(&file_a).unwrap();
    panel.load_entries().unwrap();

    assert!(
        !panel.marked.contains("a.txt"),
        "deleted file should be unmarked"
    );
    assert!(
        panel.marked.contains("b.txt"),
        "existing file should stay marked"
    );
}

#[test]
fn test_marked_files_all_removed() {
    let temp = TempDir::new().unwrap();
    let file = temp.path().join("only.txt");
    fs::write(&file, "x").unwrap();

    let mut panel = Panel::new(temp.path().to_path_buf()).unwrap();
    panel.marked.insert(FileName::from("only.txt"));

    fs::remove_file(&file).unwrap();
    panel.load_entries().unwrap();

    assert!(panel.marked.is_empty(), "all marks should be cleared");
}

// ---------------------------------------------------------------------------
// Panel selection after reload when file disappears
// ---------------------------------------------------------------------------

#[test]
fn test_selection_reset_when_selected_file_deleted() {
    let temp = TempDir::new().unwrap();
    let a = temp.path().join("a.txt");
    let b = temp.path().join("b.txt");
    fs::write(&a, "a").unwrap();
    fs::write(&b, "b").unwrap();

    let mut panel = Panel::new(temp.path().to_path_buf()).unwrap();
    // Select "b.txt"
    if let Some(pos) = panel.entries.iter().position(|e| e.name == "b.txt") {
        panel.select(Some(pos));
    }

    fs::remove_file(&b).unwrap();
    panel.load_entries().unwrap();

    // Selection should fall back to 0 since "b.txt" no longer exists
    let sel = panel.selected();
    assert!(sel.is_some(), "should have a selection");
    assert_eq!(sel.unwrap(), 0, "should select first entry as fallback");
}

#[test]
fn test_selection_preserved_when_selected_file_exists() {
    let temp = TempDir::new().unwrap();
    fs::write(temp.path().join("x.txt"), "x").unwrap();
    fs::write(temp.path().join("y.txt"), "y").unwrap();

    let mut panel = Panel::new(temp.path().to_path_buf()).unwrap();
    // Find and select "y.txt"
    let pos = panel
        .entries
        .iter()
        .position(|e| e.name == "y.txt")
        .unwrap();
    panel.select(Some(pos));

    panel.load_entries().unwrap();

    let selected = panel.get_selected_entry().unwrap();
    assert_eq!(selected.name, "y.txt", "selection should be preserved");
}

// ---------------------------------------------------------------------------
// Panel load_entries with invalid path
// ---------------------------------------------------------------------------

#[test]
fn test_panel_load_entries_invalid_path() {
    let mut panel = Panel::from_entries(
        PathBuf::from("/nonexistent/path/that/does/not/exist"),
        Vec::new(),
    );
    let result = panel.load_entries();
    assert!(result.is_err(), "loading invalid path should error");
}

// ---------------------------------------------------------------------------
// SortMode cycle and labels
// ---------------------------------------------------------------------------

#[test]
fn test_sort_mode_full_cycle() {
    let start = SortMode::Name;
    let ext = start.next();
    assert_eq!(ext, SortMode::Extension);
    let size = ext.next();
    assert_eq!(size, SortMode::Size);
    let name = size.next();
    assert_eq!(name, SortMode::Name);
}

#[test]
fn test_sort_mode_labels() {
    assert_eq!(SortMode::Name.label(), "Name");
    assert_eq!(SortMode::Extension.label(), "Ext");
    assert_eq!(SortMode::Size.label(), "Size");
}

#[test]
fn test_sort_mode_default() {
    let mode: SortMode = SortMode::default();
    assert_eq!(mode, SortMode::Name);
}

// ---------------------------------------------------------------------------
// Panel sorting
// ---------------------------------------------------------------------------

#[test]
fn test_panel_sort_by_extension() {
    let temp = TempDir::new().unwrap();
    fs::write(temp.path().join("c.rs"), "c").unwrap();
    fs::write(temp.path().join("a.txt"), "a").unwrap();
    fs::write(temp.path().join("b.md"), "b").unwrap();

    let mut panel = Panel::new(temp.path().to_path_buf()).unwrap();
    panel.sort_mode = SortMode::Extension;
    panel.load_entries().unwrap();

    let file_names: Vec<&str> = panel
        .entries
        .iter()
        .filter(|e| !e.kind.is_dir())
        .map(|e| e.name.as_str())
        .collect();
    // Extension sort: .md < .rs < .txt
    assert_eq!(file_names, vec!["b.md", "c.rs", "a.txt"]);
}

#[test]
fn test_panel_sort_by_size() {
    let temp = TempDir::new().unwrap();
    fs::write(temp.path().join("small.txt"), "x").unwrap();
    fs::write(temp.path().join("big.txt"), "x".repeat(1000)).unwrap();
    fs::write(temp.path().join("medium.txt"), "x".repeat(100)).unwrap();

    let mut panel = Panel::new(temp.path().to_path_buf()).unwrap();
    panel.sort_mode = SortMode::Size;
    panel.load_entries().unwrap();

    let file_names: Vec<&str> = panel
        .entries
        .iter()
        .filter(|e| !e.kind.is_dir())
        .map(|e| e.name.as_str())
        .collect();
    // Size sort is descending: big > medium > small
    assert_eq!(file_names, vec!["big.txt", "medium.txt", "small.txt"]);
}

// ---------------------------------------------------------------------------
// Panel navigation edge cases
// ---------------------------------------------------------------------------

#[test]
fn test_panel_next_wraps_around() {
    let temp = TempDir::new().unwrap();
    fs::write(temp.path().join("a.txt"), "a").unwrap();

    let mut panel = Panel::new(temp.path().to_path_buf()).unwrap();
    let last = panel.entries.len() - 1;
    panel.select(Some(last));

    panel.next();
    assert_eq!(panel.selected(), Some(0), "should wrap to first");
}

#[test]
fn test_panel_previous_wraps_around() {
    let temp = TempDir::new().unwrap();
    fs::write(temp.path().join("a.txt"), "a").unwrap();

    let mut panel = Panel::new(temp.path().to_path_buf()).unwrap();
    panel.select(Some(0));

    panel.previous();
    let last = panel.entries.len() - 1;
    assert_eq!(panel.selected(), Some(last), "should wrap to last");
}

#[test]
fn test_panel_next_empty_panel() {
    let temp = TempDir::new().unwrap();
    let mut panel = Panel::new(temp.path().to_path_buf()).unwrap();
    panel.entries.clear();
    panel.select(None);

    panel.next();
    // Should not panic, selection stays None
    assert_eq!(panel.selected(), None);
}

// ---------------------------------------------------------------------------
// Panel toggle_mark edge cases
// ---------------------------------------------------------------------------

#[test]
fn test_toggle_mark_dotdot_is_noop() {
    let temp = TempDir::new().unwrap();
    fs::create_dir(temp.path().join("sub")).unwrap();

    let mut panel = Panel::new(temp.path().join("sub")).unwrap();
    // Select ".." entry
    panel.select(Some(0));
    assert_eq!(panel.entries[0].name, "..");

    let result = panel.toggle_mark_selected();
    assert!(!result, "marking '..' should return false");
    assert!(panel.marked.is_empty());
}

#[test]
fn test_toggle_mark_and_unmark() {
    let temp = TempDir::new().unwrap();
    fs::write(temp.path().join("file.txt"), "x").unwrap();

    let mut panel = Panel::new(temp.path().to_path_buf()).unwrap();
    let pos = panel
        .entries
        .iter()
        .position(|e| e.name == "file.txt")
        .unwrap();
    panel.select(Some(pos));

    // Mark
    assert!(panel.toggle_mark_selected());
    assert!(panel.marked.contains("file.txt"));

    // Unmark
    assert!(panel.toggle_mark_selected());
    assert!(!panel.marked.contains("file.txt"));
}

// ---------------------------------------------------------------------------
// Panel find_first_matching / find_next_matching
// ---------------------------------------------------------------------------

#[test]
fn test_find_first_matching_case_insensitive() {
    let temp = TempDir::new().unwrap();
    fs::write(temp.path().join("README.md"), "x").unwrap();

    let mut panel = Panel::new(temp.path().to_path_buf()).unwrap();
    panel.find_first_matching("read");

    let selected = panel.get_selected_entry().unwrap();
    assert_eq!(selected.name, "README.md");
}

#[test]
fn test_find_first_matching_no_match() {
    let temp = TempDir::new().unwrap();
    fs::write(temp.path().join("a.txt"), "x").unwrap();

    let mut panel = Panel::new(temp.path().to_path_buf()).unwrap();
    let before = panel.selected();
    panel.find_first_matching("zzzzz");

    // Selection should not change
    assert_eq!(panel.selected(), before);
}

#[test]
fn test_find_next_matching_wraps() {
    let temp = TempDir::new().unwrap();
    fs::write(temp.path().join("aa.txt"), "x").unwrap();
    fs::write(temp.path().join("ab.txt"), "x").unwrap();

    let mut panel = Panel::new(temp.path().to_path_buf()).unwrap();
    panel.find_first_matching("a");
    let first = panel.selected();

    panel.find_next_matching("a");
    let second = panel.selected();
    assert_ne!(first, second, "should advance to next match");

    // Keep calling find_next until it wraps
    for _ in 0..panel.entries.len() {
        panel.find_next_matching("a");
    }
}

// ---------------------------------------------------------------------------
// Panel action_entry_names
// ---------------------------------------------------------------------------

#[test]
fn test_action_entry_names_with_marks() {
    let temp = TempDir::new().unwrap();
    fs::write(temp.path().join("a.txt"), "a").unwrap();
    fs::write(temp.path().join("b.txt"), "b").unwrap();

    let mut panel = Panel::new(temp.path().to_path_buf()).unwrap();
    panel.marked.insert(FileName::from("a.txt"));
    panel.marked.insert(FileName::from("b.txt"));

    let names = panel.action_entry_names();
    assert!(names.contains(&"a.txt".to_string()));
    assert!(names.contains(&"b.txt".to_string()));
}

#[test]
fn test_action_entry_names_no_marks_uses_selection() {
    let temp = TempDir::new().unwrap();
    fs::write(temp.path().join("only.txt"), "x").unwrap();

    let mut panel = Panel::new(temp.path().to_path_buf()).unwrap();
    let pos = panel
        .entries
        .iter()
        .position(|e| e.name == "only.txt")
        .unwrap();
    panel.select(Some(pos));

    let names = panel.action_entry_names();
    assert_eq!(names, vec!["only.txt"]);
}

// ---------------------------------------------------------------------------
// App methods
// ---------------------------------------------------------------------------

#[test]
fn test_app_switch_panel() {
    let mut app = App::new().unwrap();
    assert_eq!(app.active_panel, PanelSide::Left);

    app.switch_panel();
    assert_eq!(app.active_panel, PanelSide::Right);

    app.switch_panel();
    assert_eq!(app.active_panel, PanelSide::Left);
}

#[test]
fn test_app_get_active_panel_left() {
    let mut app = App::new().unwrap();
    app.active_panel = PanelSide::Left;
    let path = app.get_active_panel().path.clone();
    assert_eq!(path, app.left_panel.path);
}

#[test]
fn test_app_get_active_panel_right() {
    let mut app = App::new().unwrap();
    app.active_panel = PanelSide::Right;
    let path = app.get_active_panel().path.clone();
    assert_eq!(path, app.right_panel.path);
}

#[test]
fn test_app_swap_panels_exchanges_paths() {
    let temp1 = TempDir::new().unwrap();
    let temp2 = TempDir::new().unwrap();
    let mut app = App::new().unwrap();
    app.left_panel = Panel::new(temp1.path().to_path_buf()).unwrap();
    app.right_panel = Panel::new(temp2.path().to_path_buf()).unwrap();

    let left_path = app.left_panel.path.clone();
    let right_path = app.right_panel.path.clone();

    app.swap_panels();

    assert_eq!(app.left_panel.path, right_path);
    assert_eq!(app.right_panel.path, left_path);
}

// ---------------------------------------------------------------------------
// format_size edge cases
// ---------------------------------------------------------------------------

#[test]
fn test_format_size_zero() {
    assert_eq!(FileSize::from_bytes(0).to_string(), "0 B");
}

#[test]
fn test_format_size_exact_boundaries() {
    assert_eq!(FileSize::from_bytes(1023).to_string(), "1023 B");
    assert_eq!(FileSize::from_bytes(1024).to_string(), "1.00 KB");
    assert_eq!(FileSize::from_bytes(1_048_576).to_string(), "1.00 MB");
    assert_eq!(FileSize::from_bytes(1_073_741_824).to_string(), "1.00 GB");
}

#[test]
fn test_format_size_large() {
    let size = FileSize::from_bytes(u64::MAX).to_string();
    assert!(size.contains("GB") || size.contains("EB"));
}

// ---------------------------------------------------------------------------
// unique_destination edge cases
// ---------------------------------------------------------------------------

#[test]
fn test_unique_destination_no_conflict() {
    let temp = TempDir::new().unwrap();
    let result = unique_destination(temp.path(), std::ffi::OsStr::new("new.txt")).unwrap();
    assert_eq!(result, temp.path().join("new.txt"));
}

#[test]
fn test_unique_destination_with_conflict() {
    let temp = TempDir::new().unwrap();
    fs::write(temp.path().join("file.txt"), "x").unwrap();

    let result = unique_destination(temp.path(), std::ffi::OsStr::new("file.txt")).unwrap();
    assert_eq!(result, temp.path().join("file_1.txt"));
}

#[test]
fn test_unique_destination_multiple_conflicts() {
    let temp = TempDir::new().unwrap();
    fs::write(temp.path().join("f.txt"), "x").unwrap();
    fs::write(temp.path().join("f_1.txt"), "x").unwrap();
    fs::write(temp.path().join("f_2.txt"), "x").unwrap();

    let result = unique_destination(temp.path(), std::ffi::OsStr::new("f.txt")).unwrap();
    assert_eq!(result, temp.path().join("f_3.txt"));
}

#[test]
fn test_unique_destination_no_extension() {
    let temp = TempDir::new().unwrap();
    fs::write(temp.path().join("Makefile"), "x").unwrap();

    let result = unique_destination(temp.path(), std::ffi::OsStr::new("Makefile")).unwrap();
    assert_eq!(result, temp.path().join("Makefile_1"));
}

// ---------------------------------------------------------------------------
// copy_dir_all
// ---------------------------------------------------------------------------

#[test]
fn test_copy_dir_all_nested() {
    let temp = TempDir::new().unwrap();
    let src = temp.path().join("src");
    let dst = temp.path().join("dst");
    fs::create_dir_all(src.join("sub1/sub2")).unwrap();
    fs::write(src.join("root.txt"), "root").unwrap();
    fs::write(src.join("sub1/a.txt"), "a").unwrap();
    fs::write(src.join("sub1/sub2/b.txt"), "b").unwrap();

    copy_dir_all(&src, &dst).unwrap();

    assert!(dst.join("root.txt").exists());
    assert!(dst.join("sub1/a.txt").exists());
    assert!(dst.join("sub1/sub2/b.txt").exists());
    assert_eq!(fs::read_to_string(dst.join("root.txt")).unwrap(), "root");
}

#[test]
fn test_copy_dir_all_src_not_exist() {
    let temp = TempDir::new().unwrap();
    let result = copy_dir_all(&temp.path().join("nope"), &temp.path().join("dst"));
    assert!(result.is_err());
}

// ---------------------------------------------------------------------------
// UI rendering: overlay interactions
// ---------------------------------------------------------------------------

fn row_text(terminal: &Terminal<TestBackend>, y: u16) -> String {
    let buf = terminal.backend().buffer();
    let width = buf.area.width;
    let mut s = String::new();
    for x in 0..width {
        s.push_str(buf[(x, y)].symbol());
    }
    s.trim_end().to_string()
}

fn all_text(terminal: &Terminal<TestBackend>) -> String {
    let height = terminal.backend().buffer().area.height;
    let mut text = String::new();
    for y in 0..height {
        text.push_str(&row_text(terminal, y));
        text.push('\n');
    }
    text
}

#[test]
fn test_ui_with_command_line_active() {
    let backend = TestBackend::new(120, 24);
    let mut terminal = Terminal::new(backend).unwrap();
    let mut app = App::new().unwrap();
    app.set_overlay(Overlay::CommandLine("cd /tmp".into()));

    terminal.draw(|f| ui(f, &mut app)).unwrap();

    let text = all_text(&terminal);
    assert!(text.contains("cd /tmp"), "should show typed command");
    // The caret is a reversed cell rather than a `_` glyph, so its presence is
    // a question about styling, not about the symbols on the row.
    let buf = terminal.backend().buffer();
    assert!(
        (0..buf.area.width).any(|x| buf[(x, 22)].bg == Color::Yellow),
        "should show the caret when active"
    );
}

#[test]
fn test_ui_with_command_line_inactive() {
    let backend = TestBackend::new(120, 24);
    let mut terminal = Terminal::new(backend).unwrap();
    let mut app = App::new().unwrap();
    app.close_overlay();

    terminal.draw(|f| ui(f, &mut app)).unwrap();

    let cmd = row_text(&terminal, 22);
    assert!(
        cmd.contains('>'),
        "prompt should be visible even when inactive"
    );
}

#[test]
fn test_ui_help_overlay_covers_panels() {
    let backend = TestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).unwrap();
    let mut app = App::new().unwrap();
    app.set_overlay(Overlay::Help);

    terminal.draw(|f| ui(f, &mut app)).unwrap();

    let text = all_text(&terminal);
    assert!(text.contains("F1: Toggle this help"));
    assert!(text.contains("F5: Copy"));
    assert!(text.contains("F8: Delete"));
}

#[test]
fn test_ui_delete_confirm_overlay() {
    let backend = TestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).unwrap();
    let mut app = App::new().unwrap();
    app.set_overlay(Overlay::DeleteConfirm(vec![PathBuf::from("important.txt")]));

    terminal.draw(|f| ui(f, &mut app)).unwrap();

    let text = all_text(&terminal);
    assert!(text.contains("Confirm Delete"));
    assert!(text.contains("1 item(s)"));
    assert!(text.contains("important.txt"));
}

#[test]
fn test_ui_new_file_overlay() {
    let backend = TestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).unwrap();
    let mut app = App::new().unwrap();
    app.set_overlay(Overlay::NewFileInput("test.rs".to_string()));

    terminal.draw(|f| ui(f, &mut app)).unwrap();

    let text = all_text(&terminal);
    assert!(text.contains("Create New File"));
    assert!(text.contains("test.rs"));
}

#[test]
fn test_ui_viewer_esc_hint() {
    let backend = TestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).unwrap();
    let mut app = App::new().unwrap();
    app.set_overlay(Overlay::Viewer(Viewer {
        title: "notes.txt".to_string(),
        lines: vec!["Line 1".to_string()],
        scroll: ScrollTop::TOP,
    }));

    terminal.draw(|f| ui(f, &mut app)).unwrap();

    let text = all_text(&terminal);
    assert!(text.contains("Esc to close"));
    assert!(text.contains("notes.txt"));
}

#[test]
fn test_ui_console_mode_renders_cleanly() {
    let backend = TestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).unwrap();
    let mut app = App::new().unwrap();
    app.view = ViewMode::Console;

    terminal.draw(|f| ui(f, &mut app)).unwrap();

    // Most rows should be empty/black
    for y in 0..22 {
        let row = row_text(&terminal, y);
        assert!(
            row.is_empty() || row.trim().is_empty(),
            "Row {y} should be blank in console mode, got: '{row}'"
        );
    }
}

// ---------------------------------------------------------------------------
// centered_rect edge cases
// ---------------------------------------------------------------------------

#[test]
fn test_centered_rect_zero_size() {
    let base = ratatui::layout::Rect::new(0, 0, 0, 0);
    let rect = centered_rect(50, 50, base);
    assert_eq!(rect.width, 0);
    assert_eq!(rect.height, 0);
}

#[test]
fn test_centered_rect_full_size() {
    let base = ratatui::layout::Rect::new(0, 0, 80, 24);
    let rect = centered_rect(100, 100, base);
    assert_eq!(rect.width, 80);
    assert_eq!(rect.height, 24);
}

// ---------------------------------------------------------------------------
// Panel cycle_sort_mode
// ---------------------------------------------------------------------------

#[test]
fn test_panel_cycle_sort_mode() {
    let temp = TempDir::new().unwrap();
    fs::write(temp.path().join("a.txt"), "x").unwrap();

    let mut panel = Panel::new(temp.path().to_path_buf()).unwrap();
    assert_eq!(panel.sort_mode, SortMode::Name);

    panel.cycle_sort_mode();
    assert_eq!(panel.sort_mode, SortMode::Extension);

    panel.cycle_sort_mode();
    assert_eq!(panel.sort_mode, SortMode::Size);

    panel.cycle_sort_mode();
    assert_eq!(panel.sort_mode, SortMode::Name);
}

// ---------------------------------------------------------------------------
// Panel clear_marks
// ---------------------------------------------------------------------------

#[test]
fn test_panel_clear_marks() {
    let temp = TempDir::new().unwrap();
    fs::write(temp.path().join("a.txt"), "a").unwrap();
    fs::write(temp.path().join("b.txt"), "b").unwrap();

    let mut panel = Panel::new(temp.path().to_path_buf()).unwrap();
    panel.marked.insert(FileName::from("a.txt"));
    panel.marked.insert(FileName::from("b.txt"));
    assert_eq!(panel.marked.len(), 2);

    panel.clear_marks();
    assert!(panel.marked.is_empty());
}

// ---------------------------------------------------------------------------
// Panel enter() on file (not dir) is noop
// ---------------------------------------------------------------------------

#[test]
fn test_panel_enter_file_is_noop() {
    let temp = TempDir::new().unwrap();
    fs::write(temp.path().join("data.txt"), "x").unwrap();

    let mut panel = Panel::new(temp.path().to_path_buf()).unwrap();
    let pos = panel
        .entries
        .iter()
        .position(|e| e.name == "data.txt")
        .unwrap();
    panel.select(Some(pos));

    let path_before = panel.path.clone();
    panel.enter().unwrap();
    assert_eq!(
        panel.path, path_before,
        "entering a file should not change path"
    );
}

// ---------------------------------------------------------------------------
// Panel selected_path returns None for ".."
// ---------------------------------------------------------------------------

#[test]
fn test_panel_selected_path_dotdot_returns_none() {
    let temp = TempDir::new().unwrap();
    let sub = temp.path().join("sub");
    fs::create_dir(&sub).unwrap();

    let mut panel = Panel::new(sub).unwrap();
    panel.select(Some(0)); // ".."
    assert!(panel.selected_path().is_none());
}

#[test]
fn test_panel_selected_path_normal_file() {
    let temp = TempDir::new().unwrap();
    fs::write(temp.path().join("file.txt"), "x").unwrap();

    let mut panel = Panel::new(temp.path().to_path_buf()).unwrap();
    let pos = panel
        .entries
        .iter()
        .position(|e| e.name == "file.txt")
        .unwrap();
    panel.select(Some(pos));

    let path = panel.selected_path().unwrap();
    assert_eq!(path, temp.path().join("file.txt"));
}
