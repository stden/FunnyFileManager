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

mod command_line_input;
mod command_line_listing;
mod command_line_navigation;
mod delete_confirm;
mod drive_selection;
mod event_loop_keys;
mod file_transfer;
mod find_matching;
mod new_file;
mod panel_navigation;
mod panels;
mod rendering;
mod selection;
mod sorting;
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
