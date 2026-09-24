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

#[path = "far_manager_hotkeys_test/command_line.rs"]
mod command_line;
#[path = "far_manager_hotkeys_test/drive_selection.rs"]
mod drive_selection;
#[path = "far_manager_hotkeys_test/file_operations.rs"]
mod file_operations;
#[path = "far_manager_hotkeys_test/help.rs"]
mod help;
#[path = "far_manager_hotkeys_test/main_loop.rs"]
mod main_loop;
#[path = "far_manager_hotkeys_test/marks_and_panels.rs"]
mod marks_and_panels;
#[path = "far_manager_hotkeys_test/navigation.rs"]
mod navigation;
#[path = "far_manager_hotkeys_test/new_file.rs"]
mod new_file;
#[path = "far_manager_hotkeys_test/quick_search.rs"]
mod quick_search;
#[path = "far_manager_hotkeys_test/viewer.rs"]
mod viewer;

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
