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

#[path = "comprehensive_test/command_line_input.rs"]
mod command_line_input;
#[path = "comprehensive_test/command_line_listing.rs"]
mod command_line_listing;
#[path = "comprehensive_test/command_line_navigation.rs"]
mod command_line_navigation;
#[path = "comprehensive_test/delete_confirm.rs"]
mod delete_confirm;
#[path = "comprehensive_test/drive_selection.rs"]
mod drive_selection;
#[path = "comprehensive_test/event_loop_keys.rs"]
mod event_loop_keys;
#[path = "comprehensive_test/file_transfer.rs"]
mod file_transfer;
#[path = "comprehensive_test/find_matching.rs"]
mod find_matching;
#[path = "comprehensive_test/new_file.rs"]
mod new_file;
#[path = "comprehensive_test/panel_navigation.rs"]
mod panel_navigation;
#[path = "comprehensive_test/panels.rs"]
mod panels;
#[path = "comprehensive_test/rendering.rs"]
mod rendering;
#[path = "comprehensive_test/selection.rs"]
mod selection;
#[path = "comprehensive_test/sorting.rs"]
mod sorting;
#[path = "comprehensive_test/viewer.rs"]
mod viewer;

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
