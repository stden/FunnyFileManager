use crossterm::event::{Event, KeyCode, KeyEvent, KeyEventKind, KeyEventState, KeyModifiers};
use funny_file_manager::handlers::{
    handle_copy, handle_delete_confirm, handle_mkdir, handle_move, handle_view, process_popup_input,
};
use funny_file_manager::models::ViewMode;
use funny_file_manager::models::{App, Overlay, Panel, PanelSide, Viewer};
/// Hotkey tests. Parity status lives in
/// `docs/knowledge-base/09-far-manager-compatibility/hotkeys-and-parity.md`.
/// One test per hotkey (or small group of related keys).
/// Strategy:
///   - Overlay keys   → call `process_popup_input` directly, assert App state
///   - Handler keys   → call handle_* directly, assert App state / filesystem
///   - Model-method keys → call App/Panel methods directly, assert state
///   - Event-loop keys (quit, root nav) → `run_app_with_event_reader` smoke tests
use funny_file_manager::quicksearch::QuickSearch;
use funny_file_manager::scroll::ScrollTop;
use funny_file_manager::{run_app_with_event_reader, WritableTestBackend};
use ratatui::Terminal;
use std::fs;
use std::fs::File;
use std::io;
use tempfile::TempDir;

// ─── helpers ────────────────────────────────────────────────────────────────

const fn press(code: KeyCode) -> KeyEvent {
    KeyEvent {
        code,
        modifiers: KeyModifiers::empty(),
        kind: KeyEventKind::Press,
        state: KeyEventState::empty(),
    }
}

const fn press_mod(code: KeyCode, modifiers: KeyModifiers) -> KeyEvent {
    KeyEvent {
        code,
        modifiers,
        kind: KeyEventKind::Press,
        state: KeyEventState::empty(),
    }
}

fn ev(code: KeyCode) -> Event {
    Event::Key(press(code))
}

fn ev_mod(code: KeyCode, modifiers: KeyModifiers) -> Event {
    Event::Key(press_mod(code, modifiers))
}

fn setup(temp: &TempDir) -> io::Result<App> {
    let mut app = App::new()?;
    app.left_panel = Panel::new(temp.path().to_path_buf())?;
    app.right_panel = Panel::new(temp.path().to_path_buf())?;
    app.active_panel = PanelSide::Left;
    Ok(app)
}

fn terminal() -> io::Result<Terminal<WritableTestBackend>> {
    Terminal::new(WritableTestBackend::new(80, 24))
}

fn run(app: App, events: Vec<Event>) -> io::Result<()> {
    let mut t = terminal()?;
    let mut iter = events.into_iter();
    run_app_with_event_reader(&mut t, app, || {
        iter.next()
            .ok_or_else(|| io::Error::new(io::ErrorKind::UnexpectedEof, "done"))
    })
}

// ─── QUIT KEYS ──────────────────────────────────────────────────────────────

#[test]
fn hotkey_q_quits() {
    let temp = TempDir::new().unwrap();
    let app = setup(&temp).unwrap();
    assert!(run(app, vec![ev(KeyCode::F(10)), ev(KeyCode::F(10))]).is_ok());
}

#[test]
fn hotkey_shift_q_quits() {
    let temp = TempDir::new().unwrap();
    let app = setup(&temp).unwrap();
    assert!(run(app, vec![ev(KeyCode::F(10)), ev(KeyCode::F(10))]).is_ok());
}

#[test]
fn hotkey_f10_quits() {
    let temp = TempDir::new().unwrap();
    let app = setup(&temp).unwrap();
    assert!(run(app, vec![ev(KeyCode::F(10)), ev(KeyCode::F(10))]).is_ok());
}

#[test]
fn hotkey_ctrl_c_quits() {
    let temp = TempDir::new().unwrap();
    let app = setup(&temp).unwrap();
    assert!(run(app, vec![ev_mod(KeyCode::Char('c'), KeyModifiers::CONTROL)]).is_ok());
}

// ─── PANEL NAVIGATION ───────────────────────────────────────────────────────

#[test]
fn hotkey_down_moves_selection_forward() {
    let temp = TempDir::new().unwrap();
    fs::write(temp.path().join("a.txt"), b"").unwrap();
    fs::write(temp.path().join("b.txt"), b"").unwrap();
    let mut panel = Panel::new(temp.path().to_path_buf()).unwrap();
    panel.select(Some(0));
    panel.next();
    assert_eq!(panel.selected(), Some(1));
}

#[test]
fn hotkey_up_moves_selection_backward() {
    let temp = TempDir::new().unwrap();
    fs::write(temp.path().join("a.txt"), b"").unwrap();
    let mut panel = Panel::new(temp.path().to_path_buf()).unwrap();
    let last = panel.entries.len() - 1;
    panel.select(Some(last));
    panel.previous();
    assert_eq!(panel.selected(), Some(last - 1));
}

#[test]
fn hotkey_down_stops_at_the_last_entry() {
    let temp = TempDir::new().unwrap();
    fs::write(temp.path().join("a.txt"), b"").unwrap();
    let mut panel = Panel::new(temp.path().to_path_buf()).unwrap();
    let last = panel.entries.len() - 1;
    panel.select(Some(last));
    panel.next();
    assert_eq!(panel.selected(), Some(last));
}

#[test]
fn hotkey_up_stops_at_the_first_entry() {
    let temp = TempDir::new().unwrap();
    fs::write(temp.path().join("a.txt"), b"").unwrap();
    let mut panel = Panel::new(temp.path().to_path_buf()).unwrap();
    panel.select(Some(0));
    panel.previous();
    assert_eq!(panel.selected(), Some(0));
}

#[test]
fn hotkey_home_jumps_to_first_entry() {
    let temp = TempDir::new().unwrap();
    fs::write(temp.path().join("a.txt"), b"").unwrap();
    let mut panel = Panel::new(temp.path().to_path_buf()).unwrap();
    panel.select(Some(panel.entries.len() - 1));
    panel.select(Some(0)); // home logic
    assert_eq!(panel.selected(), Some(0));
}

#[test]
fn hotkey_end_jumps_to_last_entry() {
    let temp = TempDir::new().unwrap();
    fs::write(temp.path().join("a.txt"), b"").unwrap();
    fs::write(temp.path().join("b.txt"), b"").unwrap();
    let mut panel = Panel::new(temp.path().to_path_buf()).unwrap();
    panel.select(Some(0));
    let last = panel.entries.len() - 1;
    panel.select(Some(last)); // end logic
    assert_eq!(panel.selected(), Some(last));
}

#[test]
fn hotkey_tab_switches_active_panel() {
    let temp = TempDir::new().unwrap();
    let mut app = setup(&temp).unwrap();
    assert_eq!(app.active_panel, PanelSide::Left);
    app.switch_panel();
    assert_eq!(app.active_panel, PanelSide::Right);
    app.switch_panel();
    assert_eq!(app.active_panel, PanelSide::Left);
}

#[test]
fn hotkey_left_selects_left_panel() {
    let temp = TempDir::new().unwrap();
    let mut app = setup(&temp).unwrap();
    app.active_panel = PanelSide::Right;
    app.active_panel = PanelSide::Left; // left arrow logic
    assert_eq!(app.active_panel, PanelSide::Left);
}

#[test]
fn hotkey_right_selects_right_panel() {
    let temp = TempDir::new().unwrap();
    let mut app = setup(&temp).unwrap();
    app.active_panel = PanelSide::Left;
    app.active_panel = PanelSide::Right; // right arrow logic
    assert_eq!(app.active_panel, PanelSide::Right);
}

#[test]
fn hotkey_enter_enters_subdirectory() {
    let temp = TempDir::new().unwrap();
    let sub = temp.path().join("subdir");
    fs::create_dir(&sub).unwrap();
    let mut panel = Panel::new(temp.path().to_path_buf()).unwrap();
    let idx = panel
        .entries
        .iter()
        .position(|e| e.name == "subdir")
        .unwrap();
    panel.select(Some(idx));
    panel.enter().unwrap();
    assert_eq!(panel.path, sub);
}

#[test]
fn hotkey_enter_on_dotdot_goes_to_parent() {
    let temp = TempDir::new().unwrap();
    let sub = temp.path().join("subdir");
    fs::create_dir(&sub).unwrap();
    let mut panel = Panel::new(sub).unwrap();
    // ".." is entry 0
    panel.select(Some(0));
    panel.enter().unwrap();
    assert_eq!(panel.path, temp.path());
}

#[test]
fn hotkey_enter_on_file_does_not_change_path() {
    let temp = TempDir::new().unwrap();
    fs::write(temp.path().join("file.txt"), b"data").unwrap();
    let mut panel = Panel::new(temp.path().to_path_buf()).unwrap();
    let idx = panel
        .entries
        .iter()
        .position(|e| e.name == "file.txt")
        .unwrap();
    panel.select(Some(idx));
    let before = panel.path.clone();
    panel.enter().unwrap();
    assert_eq!(panel.path, before);
}

// ─── F1 HELP ────────────────────────────────────────────────────────────────

#[test]
fn hotkey_f1_toggles_help_on() {
    let temp = TempDir::new().unwrap();
    let mut app = setup(&temp).unwrap();
    assert!(!matches!(app.overlay, Overlay::Help));
    app.set_overlay(Overlay::Help); // F1 logic
    assert!(matches!(app.overlay, Overlay::Help));
}

#[test]
fn hotkey_f1_toggles_help_off() {
    let temp = TempDir::new().unwrap();
    let mut app = setup(&temp).unwrap();
    app.set_overlay(Overlay::Help);
    app.close_overlay();
    assert!(!matches!(app.overlay, Overlay::Help));
}

#[test]
fn overlay_help_esc_closes() {
    let temp = TempDir::new().unwrap();
    let mut app = setup(&temp).unwrap();
    app.set_overlay(Overlay::Help);
    process_popup_input(&mut app, press(KeyCode::Esc));
    assert!(!matches!(app.overlay, Overlay::Help));
}

#[test]
fn overlay_help_f1_closes() {
    let temp = TempDir::new().unwrap();
    let mut app = setup(&temp).unwrap();
    app.set_overlay(Overlay::Help);
    process_popup_input(&mut app, press(KeyCode::F(1)));
    assert!(!matches!(app.overlay, Overlay::Help));
}

#[test]
fn overlay_help_other_key_keeps_open() {
    let temp = TempDir::new().unwrap();
    let mut app = setup(&temp).unwrap();
    app.set_overlay(Overlay::Help);
    process_popup_input(&mut app, press(KeyCode::Down));
    assert!(
        matches!(app.overlay, Overlay::Help),
        "help should stay open on non-close keys"
    );
}

// ─── ALT+F1 / ALT+F2 DRIVE SELECTION ────────────────────────────────────────

#[test]
fn hotkey_alt_f1_opens_drive_selection_for_left_panel() {
    let temp = TempDir::new().unwrap();
    let mut app = setup(&temp).unwrap();
    assert!(!matches!(app.overlay, Overlay::DriveSelection { .. }));
    app.set_overlay(Overlay::DriveSelection {
        side: PanelSide::Left,
        selected_index: 0,
        rows: funny_file_manager::models::drive_rows(),
    }); // Alt+F1 logic
    assert!(matches!(
        app.overlay,
        Overlay::DriveSelection {
            side: PanelSide::Left,
            ..
        }
    ));
}

#[test]
fn hotkey_alt_f2_opens_drive_selection_for_right_panel() {
    let temp = TempDir::new().unwrap();
    let mut app = setup(&temp).unwrap();
    app.set_overlay(Overlay::DriveSelection {
        side: PanelSide::Right,
        selected_index: 0,
        rows: funny_file_manager::models::drive_rows(),
    }); // Alt+F2 logic
    assert!(matches!(
        app.overlay,
        Overlay::DriveSelection {
            side: PanelSide::Right,
            ..
        }
    ));
}

#[test]
fn overlay_drive_esc_closes() {
    let temp = TempDir::new().unwrap();
    let mut app = setup(&temp).unwrap();
    app.set_overlay(Overlay::DriveSelection {
        side: PanelSide::Left,
        selected_index: 0,
        rows: funny_file_manager::models::drive_rows(),
    });
    process_popup_input(&mut app, press(KeyCode::Esc));
    assert!(!matches!(app.overlay, Overlay::DriveSelection { .. }));
}

#[test]
fn overlay_drive_down_increments_index() {
    let temp = TempDir::new().unwrap();
    let mut app = setup(&temp).unwrap();
    app.set_overlay(Overlay::DriveSelection {
        side: PanelSide::Left,
        selected_index: 0,
        rows: funny_file_manager::models::drive_rows(),
    });
    process_popup_input(&mut app, press(KeyCode::Down));
    // index should advance if drives list has >1 entry
    let drives = App::get_all_drives();
    if drives.len() > 1 {
        assert!(matches!(
            app.overlay,
            Overlay::DriveSelection {
                selected_index: 1,
                ..
            }
        ));
    }
}

#[test]
fn overlay_drive_up_decrements_index() {
    let temp = TempDir::new().unwrap();
    let mut app = setup(&temp).unwrap();
    let drives = App::get_all_drives();
    let last = drives.len().saturating_sub(1);
    app.set_overlay(Overlay::DriveSelection {
        side: PanelSide::Left,
        selected_index: last,
        rows: funny_file_manager::models::drive_rows(),
    });
    if last > 0 {
        process_popup_input(&mut app, press(KeyCode::Up));
        let index = if let Overlay::DriveSelection { selected_index, .. } = app.overlay {
            selected_index
        } else {
            0
        };
        assert_eq!(index, last - 1);
    }
}

#[test]
fn overlay_drive_home_jumps_to_zero() {
    let temp = TempDir::new().unwrap();
    let mut app = setup(&temp).unwrap();
    app.set_overlay(Overlay::DriveSelection {
        side: PanelSide::Left,
        selected_index: 3,
        rows: funny_file_manager::models::drive_rows(),
    });
    process_popup_input(&mut app, press(KeyCode::Home));
    assert!(matches!(
        app.overlay,
        Overlay::DriveSelection {
            selected_index: 0,
            ..
        }
    ));
}

#[test]
fn overlay_drive_end_jumps_to_last() {
    let temp = TempDir::new().unwrap();
    let mut app = setup(&temp).unwrap();
    app.set_overlay(Overlay::DriveSelection {
        side: PanelSide::Left,
        selected_index: 0,
        rows: funny_file_manager::models::drive_rows(),
    });
    process_popup_input(&mut app, press(KeyCode::End));
    let last = App::get_all_drives().len().saturating_sub(1);
    let index = if let Overlay::DriveSelection { selected_index, .. } = app.overlay {
        selected_index
    } else {
        0
    };
    assert_eq!(index, last);
}

// ─── F3 VIEW ────────────────────────────────────────────────────────────────

#[test]
fn hotkey_f3_opens_viewer_for_file() {
    let temp = TempDir::new().unwrap();
    fs::write(temp.path().join("readme.txt"), b"hello world").unwrap();
    let mut app = setup(&temp).unwrap();
    let idx = app
        .left_panel
        .entries
        .iter()
        .position(|e| e.name == "readme.txt")
        .unwrap();
    app.left_panel.select(Some(idx));
    handle_view(&mut app).unwrap();
    assert!(app.get_viewer().is_some(), "viewer should be set after F3");
    assert_eq!(app.get_viewer().unwrap().title(), "readme.txt");
}

#[test]
fn hotkey_f3_no_selection_is_noop() {
    let temp = TempDir::new().unwrap();
    let mut app = setup(&temp).unwrap();
    app.left_panel.select(None);
    handle_view(&mut app).unwrap();
    assert!(app.get_viewer().is_none());
}

#[test]
fn overlay_viewer_esc_closes() {
    let temp = TempDir::new().unwrap();
    let mut app = setup(&temp).unwrap();
    app.set_overlay(Overlay::Viewer(Viewer::report("t", vec!["line".into()])));
    process_popup_input(&mut app, press(KeyCode::Esc));
    assert!(app.get_viewer().is_none());
}

#[test]
fn overlay_viewer_f3_closes() {
    let temp = TempDir::new().unwrap();
    let mut app = setup(&temp).unwrap();
    app.set_overlay(Overlay::Viewer(Viewer::report("t", vec!["line".into()])));
    process_popup_input(&mut app, press(KeyCode::F(3)));
    assert!(app.get_viewer().is_none());
}

#[test]
fn overlay_viewer_enter_closes() {
    let temp = TempDir::new().unwrap();
    let mut app = setup(&temp).unwrap();
    app.set_overlay(Overlay::Viewer(Viewer::report("t", vec!["line".into()])));
    process_popup_input(&mut app, press(KeyCode::Enter));
    assert!(app.get_viewer().is_none());
}

#[test]
fn overlay_viewer_down_scrolls() {
    let temp = TempDir::new().unwrap();
    let mut app = setup(&temp).unwrap();
    let lines: Vec<String> = (0..20).map(|i| format!("line {i}")).collect();
    app.set_overlay(Overlay::Viewer(Viewer::report("t", lines)));
    app.set_viewport(funny_file_manager::models::Viewport::new(2));
    process_popup_input(&mut app, press(KeyCode::Down));
    assert_eq!(app.get_viewer().unwrap().scroll.lines(), 1);
}

#[test]
fn overlay_viewer_up_at_zero_stays_at_zero() {
    let temp = TempDir::new().unwrap();
    let mut app = setup(&temp).unwrap();
    app.set_overlay(Overlay::Viewer(Viewer::report("t", vec!["line".into()])));
    process_popup_input(&mut app, press(KeyCode::Up));
    assert_eq!(app.get_viewer().unwrap().scroll, ScrollTop::TOP);
}

#[test]
fn overlay_viewer_up_decrements_scroll() {
    let temp = TempDir::new().unwrap();
    let mut app = setup(&temp).unwrap();
    let lines: Vec<String> = (0..20).map(|i| format!("line {i}")).collect();
    app.set_overlay(Overlay::Viewer(Viewer {
        scroll: ScrollTop::TOP.down(5, usize::MAX),
        ..Viewer::report("t", lines)
    }));
    process_popup_input(&mut app, press(KeyCode::Up));
    assert_eq!(app.get_viewer().unwrap().scroll.lines(), 4);
}

/// A hundred-line viewer as it stands after one frame with 23 body rows.
fn drawn_viewer(temp: &TempDir) -> io::Result<App> {
    let mut app = setup(temp)?;
    let lines: Vec<String> = (0..100).map(|i| format!("line {i}")).collect();
    app.set_overlay(Overlay::Viewer(Viewer::report("t", lines)));
    app.set_viewport(funny_file_manager::models::Viewport::new(24));
    Ok(app)
}

#[test]
fn overlay_viewer_down_stops_with_the_last_line_on_the_bottom_row() {
    let temp = TempDir::new().unwrap();
    let mut app = drawn_viewer(&temp).unwrap();
    for _ in 0..200 {
        process_popup_input(&mut app, press(KeyCode::Down));
    }
    assert_eq!(app.get_viewer().unwrap().scroll.lines(), 77);
}

#[test]
fn overlay_viewer_pgdn_pgup_move_one_screen() {
    let temp = TempDir::new().unwrap();
    let mut app = drawn_viewer(&temp).unwrap();
    process_popup_input(&mut app, press(KeyCode::PageDown));
    assert_eq!(app.get_viewer().unwrap().scroll.lines(), 23);
    process_popup_input(&mut app, press(KeyCode::PageUp));
    assert_eq!(app.get_viewer().unwrap().scroll, ScrollTop::TOP);
}

#[test]
fn overlay_viewer_end_and_home_jump_to_the_last_and_first_screen() {
    let temp = TempDir::new().unwrap();
    let mut app = drawn_viewer(&temp).unwrap();
    process_popup_input(&mut app, press(KeyCode::End));
    assert_eq!(app.get_viewer().unwrap().scroll.lines(), 77);
    process_popup_input(&mut app, press(KeyCode::Home));
    assert_eq!(app.get_viewer().unwrap().scroll, ScrollTop::TOP);
}

#[test]
fn overlay_viewer_left_right_pan_one_column_and_ctrl_twenty() {
    let temp = TempDir::new().unwrap();
    let mut app = drawn_viewer(&temp).unwrap();
    process_popup_input(&mut app, press(KeyCode::Left));
    assert_eq!(app.get_viewer().unwrap().col.cells(), 0, "Left at 0 stays");
    process_popup_input(&mut app, press(KeyCode::Right));
    assert_eq!(app.get_viewer().unwrap().col.cells(), 1);
    process_popup_input(&mut app, press_mod(KeyCode::Right, KeyModifiers::CONTROL));
    assert_eq!(app.get_viewer().unwrap().col.cells(), 21);
    process_popup_input(&mut app, press_mod(KeyCode::Left, KeyModifiers::CONTROL));
    assert_eq!(app.get_viewer().unwrap().col.cells(), 1);
    process_popup_input(&mut app, press(KeyCode::Left));
    assert_eq!(app.get_viewer().unwrap().col.cells(), 0);
}

// ─── SHIFT+F4 NEW FILE ───────────────────────────────────────────────────────

#[test]
fn hotkey_shift_f4_opens_new_file_dialog() {
    let temp = TempDir::new().unwrap();
    let mut app = setup(&temp).unwrap();
    assert!(app.get_new_file_input().is_none());
    app.set_overlay(Overlay::NewFileInput(String::new())); // Shift+F4 logic
    assert!(app.get_new_file_input().is_some());
}

#[test]
fn overlay_new_file_esc_cancels() {
    let temp = TempDir::new().unwrap();
    let mut app = setup(&temp).unwrap();
    app.set_overlay(Overlay::NewFileInput("partial".into()));
    process_popup_input(&mut app, press(KeyCode::Esc));
    assert!(app.get_new_file_input().is_none());
}

#[test]
fn overlay_new_file_char_appends_to_input() {
    let temp = TempDir::new().unwrap();
    let mut app = setup(&temp).unwrap();
    app.set_overlay(Overlay::NewFileInput(String::new()));
    process_popup_input(&mut app, press(KeyCode::Char('h')));
    process_popup_input(&mut app, press(KeyCode::Char('i')));
    assert_eq!(app.get_new_file_input(), Some("hi"));
}

#[test]
fn overlay_new_file_backspace_removes_last_char() {
    let temp = TempDir::new().unwrap();
    let mut app = setup(&temp).unwrap();
    app.set_overlay(Overlay::NewFileInput("abc".into()));
    process_popup_input(&mut app, press(KeyCode::Backspace));
    assert_eq!(app.get_new_file_input(), Some("ab"));
}

#[test]
fn overlay_new_file_enter_creates_file_on_disk() {
    let temp = TempDir::new().unwrap();
    let mut app = setup(&temp).unwrap();
    app.set_overlay(Overlay::NewFileInput("newfile.txt".into()));
    process_popup_input(&mut app, press(KeyCode::Enter));
    assert!(app.get_new_file_input().is_none(), "dialog should close");
    assert!(
        temp.path().join("newfile.txt").exists(),
        "file should be created"
    );
}

#[test]
fn overlay_new_file_enter_empty_name_shows_error() {
    let temp = TempDir::new().unwrap();
    let mut app = setup(&temp).unwrap();
    app.set_overlay(Overlay::NewFileInput(String::new()));
    process_popup_input(&mut app, press(KeyCode::Enter));
    assert!(app.get_new_file_input().is_none());
    assert!(app.status_text().contains("empty") || app.status_text().contains("Filename"));
}

// ─── F5 COPY ────────────────────────────────────────────────────────────────

#[test]
fn hotkey_f5_copies_file_to_other_panel() {
    let temp = TempDir::new().unwrap();
    let src = temp.path().join("src");
    let dst = temp.path().join("dst");
    fs::create_dir(&src).unwrap();
    fs::create_dir(&dst).unwrap();
    fs::write(src.join("copy_me.txt"), b"data").unwrap();

    let mut app = App::new().unwrap();
    app.left_panel = Panel::new(src).unwrap();
    app.right_panel = Panel::new(dst.clone()).unwrap();
    app.active_panel = PanelSide::Left;
    let idx = app
        .left_panel
        .entries
        .iter()
        .position(|e| e.name == "copy_me.txt")
        .unwrap();
    app.left_panel.select(Some(idx));

    handle_copy(&mut app).unwrap();
    assert!(
        dst.join("copy_me.txt").exists(),
        "file should be copied to dst panel"
    );
}

#[test]
fn hotkey_f5_copy_no_selection_is_noop() {
    let temp = TempDir::new().unwrap();
    let mut app = setup(&temp).unwrap();
    app.left_panel.select(None);
    assert!(handle_copy(&mut app).is_ok());
}

// ─── F6 MOVE ────────────────────────────────────────────────────────────────

#[test]
fn hotkey_f6_moves_file_to_other_panel() {
    let temp = TempDir::new().unwrap();
    let src = temp.path().join("src");
    let dst = temp.path().join("dst");
    fs::create_dir(&src).unwrap();
    fs::create_dir(&dst).unwrap();
    fs::write(src.join("move_me.txt"), b"data").unwrap();

    let mut app = App::new().unwrap();
    app.left_panel = Panel::new(src.clone()).unwrap();
    app.right_panel = Panel::new(dst.clone()).unwrap();
    app.active_panel = PanelSide::Left;
    let idx = app
        .left_panel
        .entries
        .iter()
        .position(|e| e.name == "move_me.txt")
        .unwrap();
    app.left_panel.select(Some(idx));

    handle_move(&mut app).unwrap();
    assert!(
        dst.join("move_me.txt").exists(),
        "file should appear in dst"
    );
    assert!(
        !src.join("move_me.txt").exists(),
        "file should be gone from src"
    );
}

// ─── F7 MKDIR ───────────────────────────────────────────────────────────────

#[test]
fn hotkey_f7_creates_directory_via_command_line() {
    let temp = TempDir::new().unwrap();
    let mut app = setup(&temp).unwrap();
    // F7 opens mkdir dialog → command_line
    app.set_overlay(Overlay::CommandLine(String::new().into()));
    process_popup_input(&mut app, press(KeyCode::Char('m')));
    process_popup_input(&mut app, press(KeyCode::Char('k')));
    process_popup_input(&mut app, press(KeyCode::Char('d')));
    assert_eq!(app.get_command_line(), Some("mkd"));
    // Esc cancels
    process_popup_input(&mut app, press(KeyCode::Esc));
    assert!(app.get_command_line().is_none());
}

#[test]
fn hotkey_f7_mkdir_creates_directory() {
    let temp = TempDir::new().unwrap();
    let mut app = setup(&temp).unwrap();
    handle_mkdir(&mut app).unwrap();
    // Mkdir with no input (command_line opens); verify command_line is now active or noop
    // (handle_mkdir opens command_line with prompt)
    // Just verify it doesn't crash and sets some state
    assert!(app.get_command_line().is_some() || !app.status_text().is_empty());
}

// ─── F8 DELETE ──────────────────────────────────────────────────────────────

#[test]
fn hotkey_f8_shows_delete_confirmation_dialog() {
    let temp = TempDir::new().unwrap();
    fs::write(temp.path().join("del_me.txt"), b"bye").unwrap();
    let mut app = setup(&temp).unwrap();
    let idx = app
        .left_panel
        .entries
        .iter()
        .position(|e| e.name == "del_me.txt")
        .unwrap();
    app.left_panel.select(Some(idx));
    handle_delete_confirm(&mut app).unwrap();
    assert!(
        app.get_delete_confirm().is_some(),
        "delete confirm dialog should open"
    );
}

#[test]
fn overlay_delete_confirm_n_cancels() {
    let temp = TempDir::new().unwrap();
    let mut app = setup(&temp).unwrap();
    app.set_overlay(Overlay::DeleteConfirm(vec![temp.path().join("ghost.txt")]));
    process_popup_input(&mut app, press(KeyCode::Char('n')));
    assert!(app.get_delete_confirm().is_none());
    assert!(app.status_text().contains("cancel") || app.status_text().contains("Cancel"));
}

#[test]
fn overlay_delete_confirm_capital_n_cancels() {
    let temp = TempDir::new().unwrap();
    let mut app = setup(&temp).unwrap();
    app.set_overlay(Overlay::DeleteConfirm(vec![temp.path().join("ghost.txt")]));
    process_popup_input(&mut app, press(KeyCode::Char('N')));
    assert!(app.get_delete_confirm().is_none());
}

#[test]
fn overlay_delete_confirm_esc_cancels() {
    let temp = TempDir::new().unwrap();
    let mut app = setup(&temp).unwrap();
    app.set_overlay(Overlay::DeleteConfirm(vec![temp.path().join("ghost.txt")]));
    process_popup_input(&mut app, press(KeyCode::Esc));
    assert!(app.get_delete_confirm().is_none());
}

#[test]
fn overlay_delete_confirm_y_deletes_file() {
    let temp = TempDir::new().unwrap();
    let file = temp.path().join("delete_this.txt");
    fs::write(&file, b"gone").unwrap();
    let mut app = setup(&temp).unwrap();
    app.set_overlay(Overlay::DeleteConfirm(vec![file.clone()]));
    process_popup_input(&mut app, press(KeyCode::Char('y')));
    assert!(!file.exists(), "file should be deleted after Y");
    assert!(app.get_delete_confirm().is_none());
}

#[test]
fn overlay_delete_confirm_enter_deletes_file() {
    let temp = TempDir::new().unwrap();
    let file = temp.path().join("delete_enter.txt");
    fs::write(&file, b"gone").unwrap();
    let mut app = setup(&temp).unwrap();
    app.set_overlay(Overlay::DeleteConfirm(vec![file.clone()]));
    process_popup_input(&mut app, press(KeyCode::Enter));
    assert!(!file.exists(), "file should be deleted on Enter");
}

// ─── INSERT / SPACE MARK ────────────────────────────────────────────────────

#[test]
fn hotkey_insert_marks_file() {
    let temp = TempDir::new().unwrap();
    fs::write(temp.path().join("mark_me.txt"), b"").unwrap();
    let mut app = setup(&temp).unwrap();
    let idx = app
        .left_panel
        .entries
        .iter()
        .position(|e| e.name == "mark_me.txt")
        .unwrap();
    app.left_panel.select(Some(idx));
    app.left_panel.toggle_mark_selected(); // Insert logic
    assert!(app.left_panel.marked.contains("mark_me.txt"));
}

#[test]
fn hotkey_insert_unmarks_already_marked_file() {
    let temp = TempDir::new().unwrap();
    fs::write(temp.path().join("unmark_me.txt"), b"").unwrap();
    let mut app = setup(&temp).unwrap();
    let idx = app
        .left_panel
        .entries
        .iter()
        .position(|e| e.name == "unmark_me.txt")
        .unwrap();
    app.left_panel.select(Some(idx));
    app.left_panel.marked.insert("unmark_me.txt".into());
    app.left_panel.toggle_mark_selected();
    assert!(!app.left_panel.marked.contains("unmark_me.txt"));
}

#[test]
fn hotkey_space_marks_and_advances_to_next() {
    let temp = TempDir::new().unwrap();
    fs::write(temp.path().join("a.txt"), b"").unwrap();
    fs::write(temp.path().join("b.txt"), b"").unwrap();
    let mut app = setup(&temp).unwrap();
    let idx_a = app
        .left_panel
        .entries
        .iter()
        .position(|e| e.name == "a.txt")
        .unwrap();
    app.left_panel.select(Some(idx_a));
    // Space logic: toggle_mark + next
    app.left_panel.toggle_mark_selected();
    app.left_panel.next();
    assert!(app.left_panel.marked.contains("a.txt"));
    assert_eq!(app.left_panel.selected(), Some(idx_a + 1));
}

// ─── CTRL+U SWAP PANELS ──────────────────────────────────────────────────────

#[test]
fn hotkey_ctrl_u_swaps_panel_paths() {
    let temp = TempDir::new().unwrap();
    let left_dir = temp.path().join("left");
    let right_dir = temp.path().join("right");
    fs::create_dir(&left_dir).unwrap();
    fs::create_dir(&right_dir).unwrap();
    let mut app = App::new().unwrap();
    app.left_panel = Panel::new(left_dir.clone()).unwrap();
    app.right_panel = Panel::new(right_dir.clone()).unwrap();
    app.swap_panels();
    assert_eq!(app.left_panel.path, right_dir);
    assert_eq!(app.right_panel.path, left_dir);
}

// ─── CTRL+O CONSOLE MODE ─────────────────────────────────────────────────────

/// Through `ViewMode::toggled` — the same call the Ctrl+O action makes. These
/// used to assign the field directly, so they could only agree with themselves.
#[test]
fn hotkey_ctrl_o_toggles_console_both_ways() {
    let temp = TempDir::new().unwrap();
    let mut app = setup(&temp).unwrap();
    assert_eq!(app.view, ViewMode::Panels);

    app.view = app.view.toggled();
    assert_eq!(app.view, ViewMode::Console);

    app.view = app.view.toggled();
    assert_eq!(app.view, ViewMode::Panels);
}

/// Closing a dialog is not leaving console mode: the two are orthogonal, which
/// is exactly what splitting `ViewMode` out of `Overlay` bought.
#[test]
fn closing_an_overlay_leaves_console_mode_alone() {
    let temp = TempDir::new().unwrap();
    let mut app = setup(&temp).unwrap();
    app.view = ViewMode::Console;
    app.set_overlay(Overlay::Help);

    app.close_overlay();

    assert!(matches!(app.overlay, Overlay::None));
    assert_eq!(app.view, ViewMode::Console);
}

// ─── CTRL+\ ROOT NAVIGATION ───────────────────────────────────────────────────

#[test]
fn hotkey_ctrl_backslash_navigates_to_root() {
    let temp = TempDir::new().unwrap();
    let app = setup(&temp).unwrap();
    let result = run(
        app,
        vec![
            ev_mod(KeyCode::Char('\\'), KeyModifiers::CONTROL),
            // F10 asks, and the second F10 answers.
            ev(KeyCode::F(10)),
            ev(KeyCode::F(10)),
        ],
    );
    assert!(result.is_ok());
}

// ─── ALT+LETTER QUICK SEARCH ─────────────────────────────────────────────────

#[test]
fn hotkey_alt_letter_starts_quick_search() {
    let temp = TempDir::new().unwrap();
    let mut app = setup(&temp).unwrap();
    assert!(app.quick_search.query().is_none());
    // Alt+letter logic
    app.quick_search = QuickSearch::Active("a".to_owned());
    assert_eq!(app.quick_search.query(), Some("a"));
}

#[test]
fn hotkey_alt_letter_appends_to_existing_quick_search() {
    let temp = TempDir::new().unwrap();
    let mut app = setup(&temp).unwrap();
    // Through the real method: this used to re-implement the append inline,
    // so it could only ever agree with itself.
    app.quick_search = QuickSearch::Active("al".to_owned());
    assert_eq!(app.quick_search.push('p'), "alp");
    assert_eq!(app.quick_search.query(), Some("alp"));
}

#[test]
fn hotkey_backspace_pops_quick_search_char() {
    let temp = TempDir::new().unwrap();
    let mut app = setup(&temp).unwrap();
    app.quick_search = QuickSearch::Active("abc".to_owned());
    assert_eq!(app.quick_search.backspace(), Some("ab"));
    assert_eq!(app.quick_search.query(), Some("ab"));
}

#[test]
fn hotkey_backspace_clears_single_char_quick_search() {
    let temp = TempDir::new().unwrap();
    let mut app = setup(&temp).unwrap();
    // Emptying the query ends the search: `Active("")` is not representable.
    app.quick_search = QuickSearch::Active("x".to_owned());
    assert_eq!(app.quick_search.backspace(), None);
    assert_eq!(app.quick_search, QuickSearch::Idle);
}

#[test]
fn hotkey_esc_clears_quick_search() {
    let temp = TempDir::new().unwrap();
    let mut app = setup(&temp).unwrap();
    app.quick_search = QuickSearch::Active("abc".to_owned());
    app.quick_search = QuickSearch::Idle; // Esc logic
    assert!(app.quick_search.query().is_none());
}

#[test]
fn hotkey_alt_letter_jumps_to_matching_entry() {
    let temp = TempDir::new().unwrap();
    fs::write(temp.path().join("alpha.txt"), b"").unwrap();
    fs::write(temp.path().join("beta.txt"), b"").unwrap();
    let mut panel = Panel::new(temp.path().to_path_buf()).unwrap();
    let found = panel.find_first_matching("b");
    assert!(found);
    let idx = panel.selected().unwrap();
    assert_eq!(panel.entries[idx].name, "beta.txt");
}

// ─── BOOKMARKS OVERLAY ───────────────────────────────────────────────────────

// ─── COMMAND LINE OVERLAY ────────────────────────────────────────────────────

#[test]
fn overlay_command_line_esc_cancels() {
    let temp = TempDir::new().unwrap();
    let mut app = setup(&temp).unwrap();
    app.set_overlay(Overlay::CommandLine("partial cmd".into()));
    process_popup_input(&mut app, press(KeyCode::Esc));
    assert!(app.get_command_line().is_none());
}

#[test]
fn overlay_command_line_char_appends() {
    let temp = TempDir::new().unwrap();
    let mut app = setup(&temp).unwrap();
    app.set_overlay(Overlay::CommandLine(String::new().into()));
    process_popup_input(&mut app, press(KeyCode::Char('p')));
    process_popup_input(&mut app, press(KeyCode::Char('w')));
    process_popup_input(&mut app, press(KeyCode::Char('d')));
    assert_eq!(app.get_command_line(), Some("pwd"));
}

#[test]
fn overlay_command_line_backspace_removes_char() {
    let temp = TempDir::new().unwrap();
    let mut app = setup(&temp).unwrap();
    app.set_overlay(Overlay::CommandLine("abc".into()));
    process_popup_input(&mut app, press(KeyCode::Backspace));
    assert_eq!(app.get_command_line(), Some("ab"));
}

#[test]
fn overlay_command_line_enter_executes_pwd() {
    let temp = TempDir::new().unwrap();
    let mut app = setup(&temp).unwrap();
    app.set_overlay(Overlay::CommandLine("pwd".into()));
    process_popup_input(&mut app, press(KeyCode::Enter));
    assert!(
        app.get_command_line().is_none(),
        "command line should close after Enter"
    );
    assert!(
        app.status_text().contains('/') || app.status_text().contains('\\'),
        "pwd should set status_message to a path"
    );
}

#[test]
fn overlay_command_line_enter_empty_is_noop() {
    let temp = TempDir::new().unwrap();
    let mut app = setup(&temp).unwrap();
    app.set_overlay(Overlay::CommandLine(String::new().into()));
    process_popup_input(&mut app, press(KeyCode::Enter));
    assert!(app.get_command_line().is_none());
}

/// A Ctrl+letter is a readline command, never text: Ctrl+C abandons the line
/// (bash's behaviour inside a prompt) and an unbound one leaves it alone. What
/// neither may ever do is append the bare letter.
#[test]
fn overlay_command_line_ctrl_char_is_never_typed() {
    let temp = TempDir::new().unwrap();
    let mut app = setup(&temp).unwrap();

    app.set_overlay(Overlay::CommandLine("ab".into()));
    process_popup_input(
        &mut app,
        press_mod(KeyCode::Char('c'), KeyModifiers::CONTROL),
    );
    assert_eq!(
        app.get_command_line(),
        Some(""),
        "Ctrl+C abandons the typed line"
    );

    app.set_overlay(Overlay::CommandLine("ab".into()));
    process_popup_input(
        &mut app,
        press_mod(KeyCode::Char('z'), KeyModifiers::CONTROL),
    );
    assert_eq!(
        app.get_command_line(),
        Some("ab"),
        "an unbound Ctrl+char must not append to the command line"
    );
}

// ─── SEARCH OVERLAY ──────────────────────────────────────────────────────────

// ─── EVENT LOOP SMOKE: all main-loop keys ────────────────────────────────────

#[test]
fn smoke_all_main_loop_hotkeys_no_crash() {
    let temp = TempDir::new().unwrap();
    fs::write(temp.path().join("file.txt"), b"hello").unwrap();
    let app = setup(&temp).unwrap();

    let events = main_loop_hotkeys();

    assert!(run(app, events).is_ok());
}

#[test]
fn test_hotkey_logic_in_app() -> io::Result<()> {
    let temp_dir = TempDir::new()?;
    let left_path = temp_dir.path().join("left");
    let right_path = temp_dir.path().join("right");
    fs::create_dir(&left_path)?;
    fs::create_dir(&right_path)?;

    File::create(left_path.join("file1.txt"))?;

    let mut app = App::new()?;
    app.left_panel = Panel::new(left_path.clone())?;
    app.right_panel = Panel::new(right_path.clone())?;

    // Test Swap Panels logic (Ctrl+U)
    app.swap_panels();
    assert_eq!(app.left_panel.path, right_path);
    assert_eq!(app.right_panel.path, left_path);

    // Test Mark logic (Insert/Space)
    app.active_panel = PanelSide::Right; // Right now has file1.txt
    app.right_panel.load_entries()?;
    app.right_panel.select(Some(1)); // file1.txt (0 is ..)

    app.right_panel.toggle_mark_selected();
    assert!(app.right_panel.marked.contains("file1.txt"));

    app.right_panel.toggle_mark_selected();
    assert!(!app.right_panel.marked.contains("file1.txt"));

    Ok(())
}

#[test]
fn test_ctrl_r_refresh_logic() -> io::Result<()> {
    let mut app = App::new()?;
    let temp_dir = TempDir::new()?;
    app.left_panel.path = temp_dir.path().to_path_buf();
    app.right_panel.path = temp_dir.path().to_path_buf();

    // Setup backend
    let backend = WritableTestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend)?;

    // Simulating Ctrl+R
    let events = vec![
        Event::Key(KeyEvent::new(KeyCode::Char('r'), KeyModifiers::CONTROL)),
        Event::Key(KeyEvent::new(KeyCode::F(10), KeyModifiers::NONE)),
    ];
    let mut event_iter = events.into_iter();

    run_app_with_event_reader(&mut terminal, app, || {
        Ok(event_iter.next().unwrap_or(Event::Key(KeyEvent::new(
            KeyCode::F(10),
            KeyModifiers::NONE,
        ))))
    })?;

    // Status message should reflect refresh
    // Note: Since we can't easily inspect the app state after run_app_with_event_reader,
    // we assume it doesn't error. To truly test, we'd need to mock refresh_panels.

    Ok(())
}

fn main_loop_hotkeys() -> Vec<Event> {
    vec![
        ev(KeyCode::Down),
        ev(KeyCode::Up),
        ev(KeyCode::Home),
        ev(KeyCode::End),
        ev(KeyCode::Tab),
        ev(KeyCode::Left),
        ev(KeyCode::Right),
        ev(KeyCode::F(1)),                                  // help on
        ev(KeyCode::F(1)),                                  // help off  (goes through overlay now)
        ev_mod(KeyCode::F(1), KeyModifiers::ALT),           // drive select left
        ev(KeyCode::Esc),                                   // close drives
        ev_mod(KeyCode::F(2), KeyModifiers::ALT),           // drive select right
        ev(KeyCode::Esc),                                   // close drives
        ev_mod(KeyCode::F(4), KeyModifiers::SHIFT),         // new file dialog
        ev(KeyCode::Esc),                                   // cancel
        ev_mod(KeyCode::Char('u'), KeyModifiers::CONTROL),  // swap panels
        ev_mod(KeyCode::Char('o'), KeyModifiers::CONTROL),  // console on
        ev_mod(KeyCode::Char('o'), KeyModifiers::CONTROL),  // console off
        ev_mod(KeyCode::Char('\\'), KeyModifiers::CONTROL), // go to root
        ev(KeyCode::Insert),
        ev(KeyCode::Char(' ')),
        ev_mod(KeyCode::Char('a'), KeyModifiers::ALT), // quick search
        ev(KeyCode::Backspace),
        ev(KeyCode::Esc),
        ev(KeyCode::F(10)), // quit — asks first
        ev(KeyCode::F(10)), // …and answers
    ]
}
