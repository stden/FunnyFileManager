//! Purpose: Exercises broad application workflows that span models, handlers, and UI state.
//!
use crossterm::event::{Event, KeyCode, KeyEvent, KeyEventKind, KeyEventState, KeyModifiers};
use funny_file_manager::models::EntryKind;
use funny_file_manager::models::FileName;
use funny_file_manager::models::FileSize;
use funny_file_manager::models::Modified;
use funny_file_manager::models::ViewMode;
use funny_file_manager::models::{
    App, DirEntry, DriveEntry, Overlay, Panel, PanelSide, SortMode, Viewer,
};
use funny_file_manager::scroll::ScrollTop;
use funny_file_manager::{run_app_with_event_reader, WritableTestBackend};
use ratatui::Terminal;
use std::fs;
use std::io;
use std::path::PathBuf;
use tempfile::TempDir;

const fn key(code: KeyCode) -> Event {
    Event::Key(KeyEvent {
        code,
        modifiers: KeyModifiers::empty(),
        kind: KeyEventKind::Press,
        state: KeyEventState::empty(),
    })
}

const fn key_mod(code: KeyCode, modifiers: KeyModifiers) -> Event {
    Event::Key(KeyEvent {
        code,
        modifiers,
        kind: KeyEventKind::Press,
        state: KeyEventState::empty(),
    })
}

const fn key_release(code: KeyCode) -> Event {
    Event::Key(KeyEvent {
        code,
        modifiers: KeyModifiers::empty(),
        kind: KeyEventKind::Release,
        state: KeyEventState::empty(),
    })
}

fn setup_app(temp: &TempDir) -> io::Result<(App, Terminal<WritableTestBackend>)> {
    let mut app = App::new()?;
    app.left_panel = Panel::new(temp.path().to_path_buf())?;
    app.right_panel = Panel::new(temp.path().to_path_buf())?;
    let backend = WritableTestBackend::new(80, 24);
    let terminal = Terminal::new(backend)?;
    Ok((app, terminal))
}

fn run_events(
    terminal: &mut Terminal<WritableTestBackend>,
    app: App,
    events: Vec<Event>,
) -> io::Result<()> {
    let mut iter = events.into_iter();
    run_app_with_event_reader(terminal, app, || {
        iter.next()
            .ok_or_else(|| io::Error::new(io::ErrorKind::UnexpectedEof, "done"))
    })
}

// ==================== Panel Navigation Edge Cases ====================

#[test]
fn test_panel_next_stops_at_the_last_entry() {
    let temp = TempDir::new().unwrap();
    fs::write(temp.path().join("a.txt"), b"").unwrap();
    fs::write(temp.path().join("b.txt"), b"").unwrap();

    let mut panel = Panel::new(temp.path().to_path_buf()).unwrap();
    // entries: ["..", "a.txt", "b.txt"]
    let last = panel.entries.len() - 1;
    panel.select(Some(last));
    panel.next();
    assert_eq!(
        panel.selected(),
        Some(last),
        "Down at the bottom stays there rather than jumping to the top"
    );
}

#[test]
fn test_panel_previous_stops_at_the_first_entry() {
    let temp = TempDir::new().unwrap();
    fs::write(temp.path().join("a.txt"), b"").unwrap();

    let mut panel = Panel::new(temp.path().to_path_buf()).unwrap();
    panel.select(Some(0));
    panel.previous();
    assert_eq!(
        panel.selected(),
        Some(0),
        "Up at the top stays there rather than jumping to the bottom",
    );
}

#[test]
fn test_panel_next_empty_panel() {
    let mut panel = Panel::from_entries(PathBuf::from("/nonexistent_test_path"), vec![]);
    panel.next(); // should not panic
    assert_eq!(panel.selected(), None);
}

#[test]
fn test_panel_previous_empty_panel() {
    let mut panel = Panel::from_entries(PathBuf::from("/nonexistent_test_path"), vec![]);
    panel.previous(); // should not panic
    assert_eq!(panel.selected(), None);
}

#[test]
fn test_panel_next_no_selection() {
    let temp = TempDir::new().unwrap();
    fs::write(temp.path().join("a.txt"), b"").unwrap();

    let mut panel = Panel::new(temp.path().to_path_buf()).unwrap();
    panel.select(None);
    panel.next();
    assert_eq!(panel.selected(), Some(0), "should select first item");
}

#[test]
fn test_panel_previous_no_selection() {
    let temp = TempDir::new().unwrap();
    fs::write(temp.path().join("a.txt"), b"").unwrap();

    let mut panel = Panel::new(temp.path().to_path_buf()).unwrap();
    panel.select(None);
    panel.previous();
    assert_eq!(panel.selected(), Some(0), "should select first item");
}

#[test]
fn test_panel_single_entry() {
    let temp = TempDir::new().unwrap();
    // Only ".." entry since path has parent
    let mut panel = Panel::new(temp.path().to_path_buf()).unwrap();
    assert!(!panel.entries.is_empty());
    panel.select(Some(0));
    panel.next();
    // With one entry, wraps back to 0
    assert_eq!(panel.selected(), Some(0));
}

// ==================== Enter on File vs Directory ====================

#[test]
fn test_panel_enter_on_file_does_nothing() {
    let temp = TempDir::new().unwrap();
    fs::write(temp.path().join("file.txt"), b"data").unwrap();

    let mut panel = Panel::new(temp.path().to_path_buf()).unwrap();
    let file_idx = panel
        .entries
        .iter()
        .position(|e| e.name == "file.txt")
        .unwrap();
    panel.select(Some(file_idx));

    let path_before = panel.path.clone();
    panel.enter().unwrap();
    assert_eq!(panel.path, path_before, "path should not change for files");
}

#[test]
fn test_panel_enter_on_dotdot() {
    let temp = TempDir::new().unwrap();
    let sub = temp.path().join("sub");
    fs::create_dir(&sub).unwrap();

    let mut panel = Panel::new(sub).unwrap();
    // ".." is at index 0
    panel.select(Some(0));
    panel.enter().unwrap();
    // Should go to parent
    assert_eq!(panel.path, temp.path().to_path_buf());
}

#[test]
fn test_panel_enter_no_selection() {
    let temp = TempDir::new().unwrap();
    let mut panel = Panel::new(temp.path().to_path_buf()).unwrap();
    panel.select(None);
    // Should not panic
    panel.enter().unwrap();
}

// ==================== Mark/Unmark ====================

#[test]
fn test_toggle_mark_dotdot_returns_false() {
    let temp = TempDir::new().unwrap();
    let mut panel = Panel::new(temp.path().to_path_buf()).unwrap();
    panel.select(Some(0)); // ".."
    let result = panel.toggle_mark_selected();
    assert!(!result, "marking '..' should return false");
    assert!(panel.marked.is_empty());
}

#[test]
fn test_toggle_mark_no_selection() {
    let temp = TempDir::new().unwrap();
    let mut panel = Panel::new(temp.path().to_path_buf()).unwrap();
    panel.select(None);
    let result = panel.toggle_mark_selected();
    assert!(!result, "marking with no selection should return false");
}

#[test]
fn test_marks_cleared_on_enter_directory() {
    let temp = TempDir::new().unwrap();
    let sub = temp.path().join("sub");
    fs::create_dir(&sub).unwrap();
    fs::write(temp.path().join("file.txt"), b"").unwrap();

    let mut panel = Panel::new(temp.path().to_path_buf()).unwrap();
    // Mark a file
    let file_idx = panel
        .entries
        .iter()
        .position(|e| e.name == "file.txt")
        .unwrap();
    panel.select(Some(file_idx));
    panel.toggle_mark_selected();
    assert!(!panel.marked.is_empty());

    // Enter subdirectory
    let sub_idx = panel.entries.iter().position(|e| e.name == "sub").unwrap();
    panel.select(Some(sub_idx));
    panel.enter().unwrap();
    assert!(panel.marked.is_empty(), "marks should clear on dir change");
}

// ==================== Sort Modes ====================

#[test]
fn test_sort_mode_cycle() {
    assert_eq!(SortMode::Name.next(), SortMode::Extension);
    assert_eq!(SortMode::Extension.next(), SortMode::Size);
    assert_eq!(SortMode::Size.next(), SortMode::Name);
}

#[test]
fn test_sort_mode_labels() {
    assert_eq!(SortMode::Name.label(), "Name");
    assert_eq!(SortMode::Extension.label(), "Ext");
    assert_eq!(SortMode::Size.label(), "Size");
}

#[test]
fn test_sort_by_extension() {
    let temp = TempDir::new().unwrap();
    fs::write(temp.path().join("file.zzz"), b"").unwrap();
    fs::write(temp.path().join("file.aaa"), b"").unwrap();
    fs::write(temp.path().join("noext"), b"").unwrap();

    let mut panel = Panel::new(temp.path().to_path_buf()).unwrap();
    panel.sort_mode = SortMode::Extension;
    panel.load_entries().unwrap();

    let file_names: Vec<&str> = panel
        .entries
        .iter()
        .filter(|e| !e.kind.is_dir())
        .map(|e| e.name.as_str())
        .collect();
    // No extension should come first, then aaa, then zzz
    assert_eq!(file_names, vec!["noext", "file.aaa", "file.zzz"]);
}

#[test]
fn test_sort_by_size() {
    let temp = TempDir::new().unwrap();
    fs::write(temp.path().join("small.txt"), b"a").unwrap();
    fs::write(temp.path().join("big.txt"), b"aaaaaaaaaa").unwrap();
    fs::write(temp.path().join("medium.txt"), b"aaaaa").unwrap();

    let mut panel = Panel::new(temp.path().to_path_buf()).unwrap();
    panel.sort_mode = SortMode::Size;
    panel.load_entries().unwrap();

    let file_names: Vec<&str> = panel
        .entries
        .iter()
        .filter(|e| !e.kind.is_dir())
        .map(|e| e.name.as_str())
        .collect();
    // Largest first
    assert_eq!(file_names, vec!["big.txt", "medium.txt", "small.txt"]);
}

#[test]
fn test_cycle_sort_mode_method() {
    let temp = TempDir::new().unwrap();
    let mut panel = Panel::new(temp.path().to_path_buf()).unwrap();
    assert_eq!(panel.sort_mode, SortMode::Name);
    panel.cycle_sort_mode();
    assert_eq!(panel.sort_mode, SortMode::Extension);
    panel.cycle_sort_mode();
    assert_eq!(panel.sort_mode, SortMode::Size);
    panel.cycle_sort_mode();
    assert_eq!(panel.sort_mode, SortMode::Name);
}

// ==================== Find Matching ====================

#[test]
fn test_find_first_matching_empty_prefix() {
    let temp = TempDir::new().unwrap();
    fs::write(temp.path().join("a.txt"), b"").unwrap();
    let mut panel = Panel::new(temp.path().to_path_buf()).unwrap();
    assert!(!panel.find_first_matching(""));
}

#[test]
fn test_find_first_matching_empty_panel() {
    let mut panel = Panel::from_entries(PathBuf::from("/tmp"), vec![]);
    assert!(!panel.find_first_matching("x"));
}

#[test]
fn test_find_first_matching_case_insensitive() {
    let temp = TempDir::new().unwrap();
    fs::write(temp.path().join("Alpha.txt"), b"").unwrap();
    let mut panel = Panel::new(temp.path().to_path_buf()).unwrap();
    assert!(panel.find_first_matching("alpha"));
    let selected = panel.get_selected_entry().unwrap();
    assert_eq!(selected.name, "Alpha.txt");
}

#[test]
fn test_find_next_matching_wraps() {
    let temp = TempDir::new().unwrap();
    fs::write(temp.path().join("aa.txt"), b"").unwrap();
    fs::write(temp.path().join("ab.txt"), b"").unwrap();

    let mut panel = Panel::new(temp.path().to_path_buf()).unwrap();
    // Find first "a" match
    assert!(panel.find_first_matching("a"));
    let first = panel.selected().unwrap();
    // Find next "a" match
    assert!(panel.find_next_matching("a"));
    let second = panel.selected().unwrap();
    assert_ne!(first, second);
    // Find next again should wrap back
    assert!(panel.find_next_matching("a"));
}

#[test]
fn test_find_next_matching_no_match() {
    let temp = TempDir::new().unwrap();
    fs::write(temp.path().join("a.txt"), b"").unwrap();
    let mut panel = Panel::new(temp.path().to_path_buf()).unwrap();
    assert!(!panel.find_next_matching("zzz"));
}

#[test]
fn test_find_next_matching_empty_prefix() {
    let temp = TempDir::new().unwrap();
    fs::write(temp.path().join("a.txt"), b"").unwrap();
    let mut panel = Panel::new(temp.path().to_path_buf()).unwrap();
    assert!(!panel.find_next_matching(""));
}

// ==================== selected_path ====================

#[test]
fn test_selected_path_on_dotdot_returns_none() {
    let temp = TempDir::new().unwrap();
    let mut panel = Panel::new(temp.path().to_path_buf()).unwrap();
    panel.select(Some(0)); // ".."
    assert!(panel.selected_path().is_none());
}

#[test]
fn test_selected_path_on_file() {
    let temp = TempDir::new().unwrap();
    fs::write(temp.path().join("x.txt"), b"").unwrap();
    let mut panel = Panel::new(temp.path().to_path_buf()).unwrap();
    let idx = panel
        .entries
        .iter()
        .position(|e| e.name == "x.txt")
        .unwrap();
    panel.select(Some(idx));
    let p = panel.selected_path().unwrap();
    assert_eq!(p, temp.path().join("x.txt"));
}

// ==================== action_entry_names ====================

#[test]
fn test_action_entry_names_with_marks() {
    let temp = TempDir::new().unwrap();
    fs::write(temp.path().join("a.txt"), b"").unwrap();
    fs::write(temp.path().join("b.txt"), b"").unwrap();

    let mut panel = Panel::new(temp.path().to_path_buf()).unwrap();
    panel.marked.insert(FileName::from("a.txt"));
    panel.marked.insert(FileName::from("b.txt"));
    let names = panel.action_entry_names();
    assert!(names.contains(&"a.txt".to_owned()));
    assert!(names.contains(&"b.txt".to_owned()));
}

#[test]
fn test_action_entry_names_selected_file() {
    let temp = TempDir::new().unwrap();
    fs::write(temp.path().join("a.txt"), b"").unwrap();

    let mut panel = Panel::new(temp.path().to_path_buf()).unwrap();
    let idx = panel
        .entries
        .iter()
        .position(|e| e.name == "a.txt")
        .unwrap();
    panel.select(Some(idx));
    let names = panel.action_entry_names();
    assert_eq!(names, vec!["a.txt"]);
}

#[test]
fn test_action_entry_names_dotdot_returns_empty() {
    let temp = TempDir::new().unwrap();
    let mut panel = Panel::new(temp.path().to_path_buf()).unwrap();
    panel.select(Some(0)); // ".."
    let names = panel.action_entry_names();
    assert!(names.is_empty());
}

// ==================== App Methods ====================

#[test]
fn test_switch_panel() {
    let mut app = App::new().unwrap();
    assert_eq!(app.active_panel, PanelSide::Left);
    app.switch_panel();
    assert_eq!(app.active_panel, PanelSide::Right);
    app.switch_panel();
    assert_eq!(app.active_panel, PanelSide::Left);
}

#[test]
fn test_get_active_inactive_panel() {
    let temp = TempDir::new().unwrap();
    let left = temp.path().join("left");
    let right = temp.path().join("right");
    fs::create_dir(&left).unwrap();
    fs::create_dir(&right).unwrap();

    let mut app = App::new().unwrap();
    app.left_panel = Panel::new(left.clone()).unwrap();
    app.right_panel = Panel::new(right.clone()).unwrap();
    app.active_panel = PanelSide::Left;

    assert_eq!(app.get_active_panel().path, left);
    assert_eq!(app.get_inactive_panel().path, right);

    app.active_panel = PanelSide::Right;
    assert_eq!(app.get_active_panel().path, right);
    assert_eq!(app.get_inactive_panel().path, left);
}

#[test]
fn test_swap_panels_preserves_marks() {
    let temp = TempDir::new().unwrap();
    let left = temp.path().join("left");
    let right = temp.path().join("right");
    fs::create_dir(&left).unwrap();
    fs::create_dir(&right).unwrap();
    fs::write(left.join("a.txt"), b"").unwrap();

    let mut app = App::new().unwrap();
    app.left_panel = Panel::new(left.clone()).unwrap();
    app.right_panel = Panel::new(right.clone()).unwrap();
    app.left_panel.marked.insert(FileName::from("a.txt"));

    app.swap_panels();

    // After swap, marks should move with the panel content
    assert!(app.right_panel.marked.contains("a.txt"));
    assert!(app.left_panel.marked.is_empty());
    assert_eq!(app.left_panel.path, right);
    assert_eq!(app.right_panel.path, left);
}

// ==================== DriveEntry ====================

#[test]
fn test_drive_entry_display() {
    let local = DriveEntry::Local(PathBuf::from("/home"));
    assert_eq!(local.display(), "/home");

    let s3 = DriveEntry::S3Profile("myprofile".to_owned());
    assert_eq!(s3.display(), "S3: myprofile");
}

#[test]
fn test_get_drives_returns_root() {
    let drives = App::get_drives();
    assert!(!drives.is_empty());
    #[cfg(not(windows))]
    assert!(drives.contains(&PathBuf::from("/")));
}

#[test]
fn test_get_all_drives_includes_local() {
    let all = App::get_all_drives();
    assert!(!all.is_empty());
    // At least one local drive
    assert!(all.iter().any(|d| matches!(d, DriveEntry::Local(_))));
}

// ==================== Viewer ====================

#[test]
fn test_viewer_default() {
    let v = Viewer::default();
    assert!(v.title().is_empty());
    assert!(v.lines.is_empty());
    assert_eq!(v.scroll, ScrollTop::TOP);
}

// ==================== Key Release Ignored ====================

#[test]
fn test_key_release_ignored() -> io::Result<()> {
    let temp = TempDir::new().unwrap();
    let (app, mut terminal) = setup_app(&temp)?;

    let events = vec![
        key_release(KeyCode::Down), // should be ignored
        key(KeyCode::F(10)),        // quit
        // F10 now asks before quitting; Enter answers Yes.
        key(KeyCode::Enter),
    ];

    run_events(&mut terminal, app, events)
}

// ==================== Overlay: Viewer Scroll ====================

#[test]
fn test_viewer_scroll_down_and_up() -> io::Result<()> {
    use std::fmt::Write;

    let temp = TempDir::new().unwrap();
    let mut content = String::new();
    for i in 0..50 {
        writeln!(content, "Line {i}").map_err(io::Error::other)?;
    }
    fs::write(temp.path().join("long.txt"), content.as_bytes()).unwrap();

    let (mut app, mut terminal) = setup_app(&temp)?;
    // Select the file
    let idx = app
        .left_panel
        .entries
        .iter()
        .position(|e| e.name == "long.txt")
        .unwrap();
    app.left_panel.select(Some(idx));

    let events = vec![
        key(KeyCode::F(3)),  // open viewer
        key(KeyCode::Down),  // scroll down
        key(KeyCode::Down),  // scroll down again
        key(KeyCode::Up),    // scroll up
        key(KeyCode::Enter), // close viewer
        key(KeyCode::F(10)), // quit
        // F10 now asks before quitting; Enter answers Yes.
        key(KeyCode::Enter),
    ];

    run_events(&mut terminal, app, events)
}

#[test]
fn test_viewer_scroll_up_at_zero() -> io::Result<()> {
    let temp = TempDir::new().unwrap();
    fs::write(temp.path().join("file.txt"), b"line1\nline2").unwrap();

    let (mut app, mut terminal) = setup_app(&temp)?;
    let idx = app
        .left_panel
        .entries
        .iter()
        .position(|e| e.name == "file.txt")
        .unwrap();
    app.left_panel.select(Some(idx));

    let events = vec![
        key(KeyCode::F(3)), // open viewer
        key(KeyCode::Up),   // scroll up at 0 - no-op
        key(KeyCode::Esc),  // close
        key(KeyCode::F(10)),
        // F10 now asks before quitting; Enter answers Yes.
        key(KeyCode::Enter),
    ];

    run_events(&mut terminal, app, events)
}

#[test]
fn test_viewer_close_with_f3() -> io::Result<()> {
    let temp = TempDir::new().unwrap();
    fs::write(temp.path().join("file.txt"), b"content").unwrap();

    let (mut app, mut terminal) = setup_app(&temp)?;
    let idx = app
        .left_panel
        .entries
        .iter()
        .position(|e| e.name == "file.txt")
        .unwrap();
    app.left_panel.select(Some(idx));

    let events = vec![
        key(KeyCode::F(3)), // open viewer
        key(KeyCode::F(3)), // close with F3
        key(KeyCode::F(10)),
        // F10 now asks before quitting; Enter answers Yes.
        key(KeyCode::Enter),
    ];

    run_events(&mut terminal, app, events)
}

// ==================== Overlay: Search ====================

// ==================== Overlay: Bookmarks ====================

// ==================== Overlay: Command Line ====================

#[test]
fn test_command_line_cd() -> io::Result<()> {
    let temp = TempDir::new().unwrap();
    let sub = temp.path().join("target");
    fs::create_dir(&sub).unwrap();

    let (mut app, mut terminal) = setup_app(&temp)?;
    let target_str = sub.to_string_lossy().to_string();
    app.set_overlay(Overlay::CommandLine(format!("cd {target_str}").into()));

    let events = vec![
        key(KeyCode::Enter), // execute "cd /path/to/target"
        key(KeyCode::F(10)),
        // F10 now asks before quitting; Enter answers Yes.
        key(KeyCode::Enter),
    ];

    run_events(&mut terminal, app, events)
}

#[test]
fn test_command_line_pwd() -> io::Result<()> {
    let temp = TempDir::new().unwrap();
    let (mut app, mut terminal) = setup_app(&temp)?;
    app.set_overlay(Overlay::CommandLine("pwd".into()));

    let events = vec![
        key(KeyCode::Enter), // execute pwd
        key(KeyCode::F(10)),
        // F10 now asks before quitting; Enter answers Yes.
        key(KeyCode::Enter),
    ];

    run_events(&mut terminal, app, events)
}

#[test]
fn test_command_line_sort_name() -> io::Result<()> {
    let temp = TempDir::new().unwrap();
    let (mut app, mut terminal) = setup_app(&temp)?;
    app.set_overlay(Overlay::CommandLine("sort name".into()));

    let events = vec![
        key(KeyCode::Enter),
        key(KeyCode::F(10)),
        key(KeyCode::Enter),
    ];

    run_events(&mut terminal, app, events)
}

#[test]
fn test_command_line_sort_ext() -> io::Result<()> {
    let temp = TempDir::new().unwrap();
    let (mut app, mut terminal) = setup_app(&temp)?;
    app.set_overlay(Overlay::CommandLine("sort ext".into()));

    let events = vec![
        key(KeyCode::Enter),
        key(KeyCode::F(10)),
        key(KeyCode::Enter),
    ];

    run_events(&mut terminal, app, events)
}

#[test]
fn test_command_line_sort_extension() -> io::Result<()> {
    let temp = TempDir::new().unwrap();
    let (mut app, mut terminal) = setup_app(&temp)?;
    app.set_overlay(Overlay::CommandLine("sort extension".into()));

    let events = vec![
        key(KeyCode::Enter),
        key(KeyCode::F(10)),
        key(KeyCode::Enter),
    ];

    run_events(&mut terminal, app, events)
}

#[test]
fn test_command_line_sort_size() -> io::Result<()> {
    let temp = TempDir::new().unwrap();
    let (mut app, mut terminal) = setup_app(&temp)?;
    app.set_overlay(Overlay::CommandLine("sort size".into()));

    let events = vec![
        key(KeyCode::Enter),
        key(KeyCode::F(10)),
        key(KeyCode::Enter),
    ];

    run_events(&mut terminal, app, events)
}

#[test]
fn test_command_line_sort_invalid() -> io::Result<()> {
    let temp = TempDir::new().unwrap();
    let (mut app, mut terminal) = setup_app(&temp)?;
    app.set_overlay(Overlay::CommandLine("sort blah".into()));

    let events = vec![
        key(KeyCode::Enter),
        key(KeyCode::F(10)),
        key(KeyCode::Enter),
    ];

    run_events(&mut terminal, app, events)
}

#[test]
fn test_command_line_mark_all() -> io::Result<()> {
    let temp = TempDir::new().unwrap();
    fs::write(temp.path().join("a.txt"), b"").unwrap();
    fs::write(temp.path().join("b.txt"), b"").unwrap();

    let (mut app, mut terminal) = setup_app(&temp)?;
    app.set_overlay(Overlay::CommandLine("mark all".into()));

    let events = vec![
        key(KeyCode::Enter),
        key(KeyCode::F(10)),
        key(KeyCode::Enter),
    ];

    run_events(&mut terminal, app, events)
}

#[test]
fn test_command_line_unmark() -> io::Result<()> {
    let temp = TempDir::new().unwrap();
    let (mut app, mut terminal) = setup_app(&temp)?;
    app.left_panel.marked.insert(FileName::from("a.txt"));
    app.set_overlay(Overlay::CommandLine("unmark".into()));

    let events = vec![
        key(KeyCode::Enter),
        key(KeyCode::F(10)),
        key(KeyCode::Enter),
    ];

    run_events(&mut terminal, app, events)
}

#[test]
fn test_command_line_help() -> io::Result<()> {
    let temp = TempDir::new().unwrap();
    let (mut app, mut terminal) = setup_app(&temp)?;
    app.set_overlay(Overlay::CommandLine("help".into()));

    let events = vec![
        key(KeyCode::Enter), // opens help
        key(KeyCode::Esc),   // close help
        key(KeyCode::F(10)),
        // F10 now asks before quitting; Enter answers Yes.
        key(KeyCode::Enter),
    ];

    run_events(&mut terminal, app, events)
}

#[test]
fn test_command_line_quit() -> io::Result<()> {
    let temp = TempDir::new().unwrap();
    let (mut app, mut terminal) = setup_app(&temp)?;
    app.set_overlay(Overlay::CommandLine("quit".into()));

    let events = vec![key(KeyCode::Enter)]; // quit command

    run_events(&mut terminal, app, events)
}

#[test]
fn test_command_line_exit() -> io::Result<()> {
    let temp = TempDir::new().unwrap();
    let (mut app, mut terminal) = setup_app(&temp)?;
    app.set_overlay(Overlay::CommandLine("exit".into()));

    let events = vec![key(KeyCode::Enter)]; // exit command

    run_events(&mut terminal, app, events)
}

#[test]
fn test_command_line_unknown() -> io::Result<()> {
    let temp = TempDir::new().unwrap();
    let (mut app, mut terminal) = setup_app(&temp)?;
    app.set_overlay(Overlay::CommandLine("foobar".into()));

    let events = vec![
        key(KeyCode::Enter),
        key(KeyCode::F(10)),
        key(KeyCode::Enter),
    ];

    run_events(&mut terminal, app, events)
}

#[test]
fn test_command_line_empty_enter() -> io::Result<()> {
    let temp = TempDir::new().unwrap();
    let (mut app, mut terminal) = setup_app(&temp)?;
    app.set_overlay(Overlay::CommandLine("".into()));

    let events = vec![
        key(KeyCode::Enter),
        key(KeyCode::F(10)),
        key(KeyCode::Enter),
    ];

    run_events(&mut terminal, app, events)
}

#[test]
fn test_command_line_escape() -> io::Result<()> {
    let temp = TempDir::new().unwrap();
    let (mut app, mut terminal) = setup_app(&temp)?;
    app.set_overlay(Overlay::CommandLine("some text".into()));

    let events = vec![key(KeyCode::Esc), key(KeyCode::F(10)), key(KeyCode::Enter)];

    run_events(&mut terminal, app, events)
}

#[test]
fn test_command_line_backspace() -> io::Result<()> {
    let temp = TempDir::new().unwrap();
    let (mut app, mut terminal) = setup_app(&temp)?;
    app.set_overlay(Overlay::CommandLine("ab".into()));

    let events = vec![
        key(KeyCode::Backspace), // "a"
        key(KeyCode::Backspace), // ""
        key(KeyCode::Esc),
        key(KeyCode::F(10)),
        // F10 now asks before quitting; Enter answers Yes.
        key(KeyCode::Enter),
    ];

    run_events(&mut terminal, app, events)
}

#[test]
fn test_command_line_type_chars() -> io::Result<()> {
    let temp = TempDir::new().unwrap();
    let (mut app, mut terminal) = setup_app(&temp)?;
    app.set_overlay(Overlay::CommandLine(String::new().into()));

    let events = vec![
        key(KeyCode::Char('p')),
        key(KeyCode::Char('w')),
        key(KeyCode::Char('d')),
        key(KeyCode::Enter),
        key(KeyCode::F(10)),
        // F10 now asks before quitting; Enter answers Yes.
        key(KeyCode::Enter),
    ];

    run_events(&mut terminal, app, events)
}

#[test]
fn test_command_bookmark_set_and_go() -> io::Result<()> {
    let temp = TempDir::new().unwrap();
    let (mut app, mut terminal) = setup_app(&temp)?;
    app.set_overlay(Overlay::CommandLine("bookmark set 0".into()));

    let events = vec![
        key(KeyCode::Enter), // set bookmark 0
        key(KeyCode::F(10)),
        // F10 now asks before quitting; Enter answers Yes.
        key(KeyCode::Enter),
    ];

    run_events(&mut terminal, app, events)
}

#[test]
fn test_command_bookmark_go_empty() -> io::Result<()> {
    let temp = TempDir::new().unwrap();
    let (mut app, mut terminal) = setup_app(&temp)?;
    app.set_overlay(Overlay::CommandLine("bookmark go 5".into()));

    let events = vec![
        key(KeyCode::Enter),
        key(KeyCode::F(10)),
        key(KeyCode::Enter),
    ];

    run_events(&mut terminal, app, events)
}

#[test]
fn test_command_cd_missing_arg() -> io::Result<()> {
    let temp = TempDir::new().unwrap();
    let (mut app, mut terminal) = setup_app(&temp)?;
    app.set_overlay(Overlay::CommandLine("cd".into()));

    let events = vec![
        key(KeyCode::Enter),
        key(KeyCode::F(10)),
        key(KeyCode::Enter),
    ];

    run_events(&mut terminal, app, events)
}

#[test]
fn test_command_cd_invalid_path() -> io::Result<()> {
    let temp = TempDir::new().unwrap();
    let (mut app, mut terminal) = setup_app(&temp)?;
    app.set_overlay(Overlay::CommandLine("cd /nonexistent_path_xyz_123".into()));

    let events = vec![
        key(KeyCode::Enter),
        key(KeyCode::F(10)),
        key(KeyCode::Enter),
    ];

    run_events(&mut terminal, app, events)
}

#[test]
fn test_command_mark_invalid_sub() -> io::Result<()> {
    let temp = TempDir::new().unwrap();
    let (mut app, mut terminal) = setup_app(&temp)?;
    app.set_overlay(Overlay::CommandLine("mark blah".into()));

    let events = vec![
        key(KeyCode::Enter),
        key(KeyCode::F(10)),
        key(KeyCode::Enter),
    ];

    run_events(&mut terminal, app, events)
}

// ==================== Overlay: Drive Selection ====================

#[test]
fn test_drive_selection_navigate_and_escape() -> io::Result<()> {
    let temp = TempDir::new().unwrap();
    let (app, mut terminal) = setup_app(&temp)?;

    let events = vec![
        key_mod(KeyCode::F(1), KeyModifiers::ALT), // open drive selection for left
        key(KeyCode::Down),                        // navigate down
        key(KeyCode::Up),                          // navigate up
        key(KeyCode::Home),                        // go to first
        key(KeyCode::End),                         // go to last
        key(KeyCode::Esc),                         // close
        key(KeyCode::F(10)),
        // F10 now asks before quitting; Enter answers Yes.
        key(KeyCode::Enter),
    ];

    run_events(&mut terminal, app, events)
}

#[test]
fn test_drive_selection_enter_local() -> io::Result<()> {
    let temp = TempDir::new().unwrap();
    let (app, mut terminal) = setup_app(&temp)?;

    let events = vec![
        key_mod(KeyCode::F(1), KeyModifiers::ALT), // open for left
        key(KeyCode::Enter),                       // select first drive (/)
        key(KeyCode::F(10)),
        // F10 now asks before quitting; Enter answers Yes.
        key(KeyCode::Enter),
    ];

    run_events(&mut terminal, app, events)
}

#[test]
fn test_drive_selection_right_panel() -> io::Result<()> {
    let temp = TempDir::new().unwrap();
    let (app, mut terminal) = setup_app(&temp)?;

    let events = vec![
        key_mod(KeyCode::F(2), KeyModifiers::ALT), // open for right
        key(KeyCode::Esc),
        key(KeyCode::F(10)),
        // F10 now asks before quitting; Enter answers Yes.
        key(KeyCode::Enter),
    ];

    run_events(&mut terminal, app, events)
}

#[test]
fn test_drive_selection_up_wraps() -> io::Result<()> {
    let temp = TempDir::new().unwrap();
    let (app, mut terminal) = setup_app(&temp)?;

    let events = vec![
        key_mod(KeyCode::F(1), KeyModifiers::ALT),
        key(KeyCode::Up), // at index 0, should wrap to last
        key(KeyCode::Esc),
        key(KeyCode::F(10)),
        // F10 now asks before quitting; Enter answers Yes.
        key(KeyCode::Enter),
    ];

    run_events(&mut terminal, app, events)
}

#[test]
fn test_drive_selection_down_wraps() -> io::Result<()> {
    let temp = TempDir::new().unwrap();
    let (app, mut terminal) = setup_app(&temp)?;

    let events = vec![
        key_mod(KeyCode::F(1), KeyModifiers::ALT),
        key(KeyCode::End),  // go to last
        key(KeyCode::Down), // should wrap to first
        key(KeyCode::Esc),
        key(KeyCode::F(10)),
        // F10 now asks before quitting; Enter answers Yes.
        key(KeyCode::Enter),
    ];

    run_events(&mut terminal, app, events)
}

// ==================== Overlay: Delete Confirmation ====================

#[test]
fn test_delete_confirm_yes() -> io::Result<()> {
    let temp = TempDir::new().unwrap();
    fs::write(temp.path().join("doomed.txt"), b"bye").unwrap();

    let (mut app, mut terminal) = setup_app(&temp)?;
    let idx = app
        .left_panel
        .entries
        .iter()
        .position(|e| e.name == "doomed.txt")
        .unwrap();
    app.left_panel.select(Some(idx));

    let events = vec![
        key(KeyCode::F(8)),      // trigger delete confirmation
        key(KeyCode::Char('y')), // confirm
        key(KeyCode::F(10)),
        // F10 now asks before quitting; Enter answers Yes.
        key(KeyCode::Enter),
    ];

    run_events(&mut terminal, app, events)?;
    assert!(!temp.path().join("doomed.txt").exists());
    Ok(())
}

#[test]
fn test_delete_confirm_no() -> io::Result<()> {
    let temp = TempDir::new().unwrap();
    fs::write(temp.path().join("safe.txt"), b"keep").unwrap();

    let (mut app, mut terminal) = setup_app(&temp)?;
    let idx = app
        .left_panel
        .entries
        .iter()
        .position(|e| e.name == "safe.txt")
        .unwrap();
    app.left_panel.select(Some(idx));

    let events = vec![
        key(KeyCode::F(8)),      // trigger delete confirmation
        key(KeyCode::Char('n')), // cancel
        key(KeyCode::F(10)),
        // F10 now asks before quitting; Enter answers Yes.
        key(KeyCode::Enter),
    ];

    run_events(&mut terminal, app, events)?;
    assert!(temp.path().join("safe.txt").exists());
    Ok(())
}

#[test]
fn test_delete_confirm_escape() -> io::Result<()> {
    let temp = TempDir::new().unwrap();
    fs::write(temp.path().join("safe.txt"), b"keep").unwrap();

    let (mut app, mut terminal) = setup_app(&temp)?;
    let idx = app
        .left_panel
        .entries
        .iter()
        .position(|e| e.name == "safe.txt")
        .unwrap();
    app.left_panel.select(Some(idx));

    let events = vec![
        key(KeyCode::F(8)), // trigger delete confirmation
        key(KeyCode::Esc),  // cancel
        key(KeyCode::F(10)),
        // F10 now asks before quitting; Enter answers Yes.
        key(KeyCode::Enter),
    ];

    run_events(&mut terminal, app, events)?;
    assert!(temp.path().join("safe.txt").exists());
    Ok(())
}

#[test]
fn test_delete_confirm_with_enter() -> io::Result<()> {
    let temp = TempDir::new().unwrap();
    fs::write(temp.path().join("gone.txt"), b"data").unwrap();

    let (mut app, mut terminal) = setup_app(&temp)?;
    let idx = app
        .left_panel
        .entries
        .iter()
        .position(|e| e.name == "gone.txt")
        .unwrap();
    app.left_panel.select(Some(idx));

    let events = vec![
        key(KeyCode::F(8)),  // trigger delete
        key(KeyCode::Enter), // confirm with Enter
        key(KeyCode::F(10)),
        // F10 now asks before quitting; Enter answers Yes.
        key(KeyCode::Enter),
    ];

    run_events(&mut terminal, app, events)?;
    assert!(!temp.path().join("gone.txt").exists());
    Ok(())
}

#[test]
fn test_delete_marked_files() -> io::Result<()> {
    let temp = TempDir::new().unwrap();
    fs::write(temp.path().join("a.txt"), b"").unwrap();
    fs::write(temp.path().join("b.txt"), b"").unwrap();
    fs::write(temp.path().join("c.txt"), b"").unwrap();

    let (mut app, mut terminal) = setup_app(&temp)?;
    app.left_panel.marked.insert(FileName::from("a.txt"));
    app.left_panel.marked.insert(FileName::from("b.txt"));

    let events = vec![
        key(KeyCode::F(8)),      // trigger delete for marked files
        key(KeyCode::Char('Y')), // confirm with uppercase Y
        key(KeyCode::F(10)),
        // F10 now asks before quitting; Enter answers Yes.
        key(KeyCode::Enter),
    ];

    run_events(&mut terminal, app, events)?;
    assert!(!temp.path().join("a.txt").exists());
    assert!(!temp.path().join("b.txt").exists());
    assert!(temp.path().join("c.txt").exists()); // not marked
    Ok(())
}

#[test]
fn test_delete_directory() -> io::Result<()> {
    let temp = TempDir::new().unwrap();
    let sub = temp.path().join("deleteme");
    fs::create_dir(&sub).unwrap();
    fs::write(sub.join("inside.txt"), b"").unwrap();

    let (mut app, mut terminal) = setup_app(&temp)?;
    let idx = app
        .left_panel
        .entries
        .iter()
        .position(|e| e.name == "deleteme")
        .unwrap();
    app.left_panel.select(Some(idx));

    let events = vec![
        key(KeyCode::F(8)),
        key(KeyCode::Char('y')),
        key(KeyCode::F(10)),
        // F10 now asks before quitting; Enter answers Yes.
        key(KeyCode::Enter),
    ];

    run_events(&mut terminal, app, events)?;
    assert!(!sub.exists());
    Ok(())
}

#[test]
fn test_delete_dotdot_does_nothing() -> io::Result<()> {
    let temp = TempDir::new().unwrap();
    let (mut app, mut terminal) = setup_app(&temp)?;
    app.left_panel.select(Some(0)); // ".."

    let events = vec![
        key(KeyCode::F(8)), // should do nothing for ".."
        key(KeyCode::F(10)),
        // F10 now asks before quitting; Enter answers Yes.
        key(KeyCode::Enter),
    ];

    run_events(&mut terminal, app, events)
}

// ==================== Overlay: New File Input (Shift+F4) ====================

#[test]
fn test_new_file_create() -> io::Result<()> {
    let temp = TempDir::new().unwrap();
    let (app, mut terminal) = setup_app(&temp)?;

    let events = vec![
        key_mod(KeyCode::F(4), KeyModifiers::SHIFT), // open new file dialog
        key(KeyCode::Char('n')),
        key(KeyCode::Char('e')),
        key(KeyCode::Char('w')),
        key(KeyCode::Char('.')),
        key(KeyCode::Char('t')),
        key(KeyCode::Char('x')),
        key(KeyCode::Char('t')),
        key(KeyCode::Enter), // create "new.txt"
        key(KeyCode::F(10)),
        // F10 now asks before quitting; Enter answers Yes.
        key(KeyCode::Enter),
    ];

    run_events(&mut terminal, app, events)?;
    assert!(temp.path().join("new.txt").exists());
    Ok(())
}

#[test]
fn test_new_file_empty_name() -> io::Result<()> {
    let temp = TempDir::new().unwrap();
    let (app, mut terminal) = setup_app(&temp)?;

    let events = vec![
        key_mod(KeyCode::F(4), KeyModifiers::SHIFT),
        key(KeyCode::Enter), // empty filename
        key(KeyCode::F(10)),
        // F10 now asks before quitting; Enter answers Yes.
        key(KeyCode::Enter),
    ];

    run_events(&mut terminal, app, events)
}

#[test]
fn test_new_file_escape() -> io::Result<()> {
    let temp = TempDir::new().unwrap();
    let (app, mut terminal) = setup_app(&temp)?;

    let events = vec![
        key_mod(KeyCode::F(4), KeyModifiers::SHIFT),
        key(KeyCode::Char('x')),
        key(KeyCode::Esc), // cancel
        key(KeyCode::F(10)),
        // F10 now asks before quitting; Enter answers Yes.
        key(KeyCode::Enter),
    ];

    run_events(&mut terminal, app, events)
}

#[test]
fn test_new_file_backspace() -> io::Result<()> {
    let temp = TempDir::new().unwrap();
    let (app, mut terminal) = setup_app(&temp)?;

    let events = vec![
        key_mod(KeyCode::F(4), KeyModifiers::SHIFT),
        key(KeyCode::Char('a')),
        key(KeyCode::Char('b')),
        key(KeyCode::Backspace), // remove 'b'
        key(KeyCode::Enter),     // create "a"
        key(KeyCode::F(10)),
        // F10 now asks before quitting; Enter answers Yes.
        key(KeyCode::Enter),
    ];

    run_events(&mut terminal, app, events)?;
    assert!(temp.path().join("a").exists());
    Ok(())
}

#[test]
fn test_new_file_already_exists() -> io::Result<()> {
    let temp = TempDir::new().unwrap();
    fs::write(temp.path().join("exists.txt"), b"data").unwrap();

    let (mut app, mut terminal) = setup_app(&temp)?;
    app.set_overlay(Overlay::NewFileInput("exists.txt".to_owned()));

    let events = vec![
        key(KeyCode::Enter), // try to create existing file
        key(KeyCode::F(10)),
        // F10 now asks before quitting; Enter answers Yes.
        key(KeyCode::Enter),
    ];

    run_events(&mut terminal, app, events)
}

// ==================== Console Mode (Ctrl+O) ====================

#[test]
fn test_console_mode_toggle() -> io::Result<()> {
    let temp = TempDir::new().unwrap();
    let (app, mut terminal) = setup_app(&temp)?;

    let events = vec![
        key_mod(KeyCode::Char('o'), KeyModifiers::CONTROL), // enter console mode
        key_mod(KeyCode::Char('o'), KeyModifiers::CONTROL), // exit console mode
        key(KeyCode::F(10)),
        // F10 now asks before quitting; Enter answers Yes.
        key(KeyCode::Enter),
    ];

    run_events(&mut terminal, app, events)
}

// ==================== Ctrl+U Swap Panels ====================

#[test]
fn test_ctrl_u_swap_panels_via_event_loop() -> io::Result<()> {
    let temp = TempDir::new().unwrap();
    let left = temp.path().join("left");
    let right = temp.path().join("right");
    fs::create_dir(&left).unwrap();
    fs::create_dir(&right).unwrap();

    let mut app = App::new().unwrap();
    app.left_panel = Panel::new(left).unwrap();
    app.right_panel = Panel::new(right).unwrap();
    let backend = WritableTestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).unwrap();

    let events = vec![
        key_mod(KeyCode::Char('u'), KeyModifiers::CONTROL),
        key(KeyCode::F(10)),
        // F10 now asks before quitting; Enter answers Yes.
        key(KeyCode::Enter),
    ];

    run_events(&mut terminal, app, events)
}

// ==================== Ctrl+\ Go to Root ====================

#[test]
fn test_ctrl_backslash_goes_to_root() -> io::Result<()> {
    let temp = TempDir::new().unwrap();
    let (app, mut terminal) = setup_app(&temp)?;

    let events = vec![
        key_mod(KeyCode::Char('\\'), KeyModifiers::CONTROL),
        key(KeyCode::F(10)),
        // F10 now asks before quitting; Enter answers Yes.
        key(KeyCode::Enter),
    ];

    run_events(&mut terminal, app, events)
}

// ==================== Home and End Keys ====================

#[test]
fn test_home_end_keys() -> io::Result<()> {
    let temp = TempDir::new().unwrap();
    for i in 0..10 {
        fs::write(temp.path().join(format!("file{i:02}.txt")), b"").unwrap();
    }

    let (app, mut terminal) = setup_app(&temp)?;

    let events = vec![
        key(KeyCode::End),  // go to last
        key(KeyCode::Home), // go to first
        key(KeyCode::F(10)),
        // F10 now asks before quitting; Enter answers Yes.
        key(KeyCode::Enter),
    ];

    run_events(&mut terminal, app, events)
}

// ==================== Insert and Space for Marking ====================

#[test]
fn test_insert_and_space_marking() -> io::Result<()> {
    let temp = TempDir::new().unwrap();
    fs::write(temp.path().join("a.txt"), b"").unwrap();
    fs::write(temp.path().join("b.txt"), b"").unwrap();

    let (app, mut terminal) = setup_app(&temp)?;

    let events = vec![
        key(KeyCode::Down),      // move past ".."
        key(KeyCode::Insert),    // mark and move down
        key(KeyCode::Char(' ')), // mark and move down
        key(KeyCode::F(10)),
        // F10 now asks before quitting; Enter answers Yes.
        key(KeyCode::Enter),
    ];

    run_events(&mut terminal, app, events)
}

// ==================== ensure_selection ====================

#[test]
fn test_ensure_selection_on_empty_panel() {
    let mut panel = Panel::from_entries(PathBuf::from("/tmp"), vec![]);
    panel.select(None);
    panel.ensure_selection();
    assert_eq!(panel.selected(), None, "empty panel stays None");
}

#[test]
fn test_ensure_selection_with_entries() {
    let temp = TempDir::new().unwrap();
    let mut panel = Panel::new(temp.path().to_path_buf()).unwrap();
    panel.select(None);
    panel.ensure_selection();
    assert_eq!(panel.selected(), Some(0));
}

// ==================== select_after_reload ====================

#[test]
fn test_select_after_reload_restores_position() {
    let temp = TempDir::new().unwrap();
    fs::write(temp.path().join("keep.txt"), b"").unwrap();
    fs::write(temp.path().join("other.txt"), b"").unwrap();

    let mut panel = Panel::new(temp.path().to_path_buf()).unwrap();
    let idx = panel
        .entries
        .iter()
        .position(|e| e.name == "keep.txt")
        .unwrap();
    panel.select(Some(idx));

    // Reload should restore selection to "keep.txt"
    panel.load_entries().unwrap();
    let selected = panel.get_selected_entry().unwrap();
    assert_eq!(selected.name, "keep.txt");
}

#[test]
fn test_select_after_reload_deleted_entry_falls_back() {
    let temp = TempDir::new().unwrap();
    fs::write(temp.path().join("gone.txt"), b"").unwrap();

    let mut panel = Panel::new(temp.path().to_path_buf()).unwrap();
    let idx = panel
        .entries
        .iter()
        .position(|e| e.name == "gone.txt")
        .unwrap();
    panel.select(Some(idx));

    // Delete the file and reload
    fs::remove_file(temp.path().join("gone.txt")).unwrap();
    panel.load_entries().unwrap();
    // Should fall back to index 0
    assert_eq!(panel.selected(), Some(0));
}

// ==================== load_entries cleans invalid marks ====================

#[test]
fn test_load_entries_cleans_stale_marks() {
    let temp = TempDir::new().unwrap();
    fs::write(temp.path().join("exists.txt"), b"").unwrap();

    let mut panel = Panel::new(temp.path().to_path_buf()).unwrap();
    panel.marked.insert(FileName::from("exists.txt"));
    panel.marked.insert(FileName::from("deleted.txt")); // stale mark

    panel.load_entries().unwrap();
    assert!(panel.marked.contains("exists.txt"));
    assert!(!panel.marked.contains("deleted.txt"));
}

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
    app.set_overlay(Overlay::NewFileInput("newfile.txt".to_owned()));

    terminal
        .draw(|f| funny_file_manager::ui::ui(f, &mut app))
        .unwrap();
}

#[test]
fn test_ui_render_new_file_input_empty() {
    let backend = ratatui::backend::TestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).unwrap();

    let mut app = App::new().unwrap();
    app.set_overlay(Overlay::NewFileInput(String::new()));

    terminal
        .draw(|f| funny_file_manager::ui::ui(f, &mut app))
        .unwrap();
}

// ==================== handle_view edge cases ====================

#[test]
fn test_view_empty_file() -> io::Result<()> {
    let temp = TempDir::new().unwrap();
    fs::write(temp.path().join("empty.txt"), b"").unwrap();

    let (mut app, mut terminal) = setup_app(&temp)?;
    let idx = app
        .left_panel
        .entries
        .iter()
        .position(|e| e.name == "empty.txt")
        .unwrap();
    app.left_panel.select(Some(idx));

    let events = vec![
        key(KeyCode::F(3)), // view empty file
        key(KeyCode::Esc),
        key(KeyCode::F(10)),
        // F10 now asks before quitting; Enter answers Yes.
        key(KeyCode::Enter),
    ];

    run_events(&mut terminal, app, events)
}

#[test]
fn test_view_directory_opens_the_information_panel() -> io::Result<()> {
    let temp = TempDir::new().unwrap();
    let sub = temp.path().join("subdir");
    fs::create_dir(&sub).unwrap();

    let (mut app, mut terminal) = setup_app(&temp)?;
    let idx = app
        .left_panel
        .entries
        .iter()
        .position(|e| e.name == "subdir")
        .unwrap();
    app.left_panel.select(Some(idx));

    let events = vec![
        // F3 on a directory measures it and puts the information panel up,
        // so the quit keys only reach the panels once that is dismissed.
        key(KeyCode::F(3)),
        key(KeyCode::Esc),
        key(KeyCode::F(10)),
        // F10 now asks before quitting; Enter answers Yes.
        key(KeyCode::Enter),
    ];

    run_events(&mut terminal, app, events)
}

// ==================== handle_copy / handle_move ====================

#[test]
fn test_copy_file_to_other_panel() -> io::Result<()> {
    let temp = TempDir::new().unwrap();
    let left = temp.path().join("left");
    let right = temp.path().join("right");
    fs::create_dir(&left).unwrap();
    fs::create_dir(&right).unwrap();
    fs::write(left.join("source.txt"), b"data").unwrap();

    let mut app = App::new().unwrap();
    app.left_panel = Panel::new(left.clone()).unwrap();
    app.right_panel = Panel::new(right.clone()).unwrap();
    let idx = app
        .left_panel
        .entries
        .iter()
        .position(|e| e.name == "source.txt")
        .unwrap();
    app.left_panel.select(Some(idx));
    let backend = WritableTestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).unwrap();

    let events = vec![
        // F5 opens the Copy dialog; Enter accepts the offered destination.
        key(KeyCode::F(5)),
        key(KeyCode::Enter),
        key(KeyCode::F(10)),
        key(KeyCode::Enter),
    ];

    run_events(&mut terminal, app, events)?;
    assert!(right.join("source.txt").exists());
    assert!(left.join("source.txt").exists()); // original still there
    Ok(())
}

#[test]
fn test_move_file_to_other_panel() -> io::Result<()> {
    let temp = TempDir::new().unwrap();
    let left = temp.path().join("left");
    let right = temp.path().join("right");
    fs::create_dir(&left).unwrap();
    fs::create_dir(&right).unwrap();
    fs::write(left.join("moveme.txt"), b"data").unwrap();

    let mut app = App::new().unwrap();
    app.left_panel = Panel::new(left.clone()).unwrap();
    app.right_panel = Panel::new(right.clone()).unwrap();
    let idx = app
        .left_panel
        .entries
        .iter()
        .position(|e| e.name == "moveme.txt")
        .unwrap();
    app.left_panel.select(Some(idx));
    let backend = WritableTestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).unwrap();

    let events = vec![
        // F6 opens the Rename/Move dialog; Enter accepts the offered path.
        key(KeyCode::F(6)),
        key(KeyCode::Enter),
        key(KeyCode::F(10)),
        key(KeyCode::Enter),
    ];

    run_events(&mut terminal, app, events)?;
    assert!(right.join("moveme.txt").exists());
    assert!(!left.join("moveme.txt").exists()); // original gone
    Ok(())
}

#[test]
fn test_copy_directory() -> io::Result<()> {
    let temp = TempDir::new().unwrap();
    let left = temp.path().join("left");
    let right = temp.path().join("right");
    fs::create_dir(&left).unwrap();
    fs::create_dir(&right).unwrap();
    let sub = left.join("mydir");
    fs::create_dir(&sub).unwrap();
    fs::write(sub.join("inside.txt"), b"data").unwrap();

    let mut app = App::new().unwrap();
    app.left_panel = Panel::new(left).unwrap();
    app.right_panel = Panel::new(right.clone()).unwrap();
    let idx = app
        .left_panel
        .entries
        .iter()
        .position(|e| e.name == "mydir")
        .unwrap();
    app.left_panel.select(Some(idx));
    let backend = WritableTestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).unwrap();

    let events = vec![
        // F5 opens the Copy dialog; Enter accepts the offered destination.
        key(KeyCode::F(5)),
        key(KeyCode::Enter),
        key(KeyCode::F(10)),
        key(KeyCode::Enter),
    ];

    run_events(&mut terminal, app, events)?;
    assert!(right.join("mydir").join("inside.txt").exists());
    Ok(())
}

// ==================== handle_mkdir ====================

// ==================== SortMode Default ====================

#[test]
fn test_sort_mode_default() {
    let mode: SortMode = SortMode::default();
    assert_eq!(mode, SortMode::Name);
}

// ==================== DirEntry ====================

#[test]
fn test_dir_entry_struct() {
    let entry = DirEntry {
        name: "test".to_owned(),
        kind: EntryKind::Directory,
        size: FileSize::from_bytes(0),
        modified: Modified::UNKNOWN,
        executable: false,
        ..DirEntry::default()
    };
    assert!(entry.kind.is_dir());
    assert_eq!(entry.size, FileSize::from_bytes(0));

    let file = DirEntry {
        name: "f.txt".to_owned(),
        kind: EntryKind::File,
        size: FileSize::from_bytes(42),
        modified: Modified::UNKNOWN,
        executable: false,
        ..DirEntry::default()
    };
    assert!(!file.kind.is_dir());
    assert_eq!(file.size, FileSize::from_bytes(42));
}
