use crossterm::event::{Event, KeyCode, KeyEvent, KeyEventKind, KeyEventState, KeyModifiers};
use funny_file_manager::handlers::{
    handle_delete_confirm, handle_view, open_mcp_status, process_popup_input,
};
/// Tests targeting coverage gaps identified by analysis.
/// Covers: `open_mcp_status`, `handle_view` edge cases, `get_source_paths`,
/// UI rendering (console mode, overlays), Panel edge cases, and `VirtualPath`
/// display variants.
use funny_file_manager::models::FileName;
use funny_file_manager::models::FileSize;
use funny_file_manager::models::ViewMode;
use funny_file_manager::models::{App, Overlay, Panel, PanelSide, SortMode, Viewer};
use funny_file_manager::quicksearch::QuickSearch;
#[cfg(feature = "s3")]
use funny_file_manager::s3::VirtualPath;
use funny_file_manager::scroll::ScrollTop;
use funny_file_manager::ui;
use funny_file_manager::utils::{copy_dir_all, refresh_panels, unique_destination};
use funny_file_manager::{run_app_with_event_reader, WritableTestBackend};
use ratatui::Terminal;
use std::ffi::OsStr;
use std::fs::{self, File};
use std::io::{self, Write};
use std::path::PathBuf;
use tempfile::TempDir;

#[path = "coverage_boost_test/app_state.rs"]
mod app_state;
#[path = "coverage_boost_test/drives.rs"]
mod drives;
#[path = "coverage_boost_test/event_loop_marks_search.rs"]
mod event_loop_marks_search;
#[path = "coverage_boost_test/event_loop_navigation.rs"]
mod event_loop_navigation;
#[path = "coverage_boost_test/event_loop_overlays.rs"]
mod event_loop_overlays;
#[path = "coverage_boost_test/panel_navigation.rs"]
mod panel_navigation;
#[path = "coverage_boost_test/panel_selection.rs"]
mod panel_selection;
#[path = "coverage_boost_test/popups.rs"]
mod popups;
#[path = "coverage_boost_test/rendering.rs"]
mod rendering;
#[path = "coverage_boost_test/sorting.rs"]
mod sorting;
#[path = "coverage_boost_test/status_and_view.rs"]
mod status_and_view;
#[path = "coverage_boost_test/utils.rs"]
mod utils;
#[path = "coverage_boost_test/virtual_path.rs"]
mod virtual_path;

const fn key_event(code: KeyCode) -> KeyEvent {
    KeyEvent::new(code, KeyModifiers::empty())
}

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

fn setup_app(temp: &TempDir) -> io::Result<App> {
    let mut app = App::new()?;
    app.left_panel = Panel::new(temp.path().to_path_buf())?;
    app.right_panel = Panel::new(temp.path().to_path_buf())?;
    Ok(app)
}

fn setup_terminal() -> io::Result<Terminal<WritableTestBackend>> {
    let backend = WritableTestBackend::new(80, 24);
    Terminal::new(backend)
}

/// Drive the real event loop over `events`. These tests prove the loop gets
/// through the sequence without panicking; how the session ended is not
/// what they check, so its outcome is dropped on purpose.
fn run_events(terminal: &mut Terminal<WritableTestBackend>, app: App, events: Vec<Event>) {
    let mut iter = events.into_iter();
    drop(run_app_with_event_reader(terminal, app, || {
        iter.next()
            .ok_or_else(|| io::Error::new(io::ErrorKind::UnexpectedEof, "done"))
    }));
}
