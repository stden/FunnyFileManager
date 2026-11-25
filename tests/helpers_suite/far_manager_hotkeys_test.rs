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

mod command_line;
mod drive_selection;
mod file_operations;
mod help;
mod main_loop;
mod marks_and_panels;
mod navigation;
mod new_file;
mod quick_search;
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
