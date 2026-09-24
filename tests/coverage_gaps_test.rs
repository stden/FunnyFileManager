//! Tests to cover remaining uncovered lines in handlers.rs, lib.rs, s3.rs, ui.rs

use crossterm::event::Event;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use funny_file_manager::handlers::{handle_delete_confirm, open_mcp_status, process_popup_input};
use funny_file_manager::models::{App, Overlay, Panel, PanelSide};
use funny_file_manager::quicksearch::QuickSearch;
#[cfg(feature = "s3")]
use funny_file_manager::s3::VirtualPath;
use funny_file_manager::scroll::ScrollTop;
use funny_file_manager::{run_app_with_event_reader, WritableTestBackend};
use ratatui::Terminal;
use std::fs::{self, File};
use std::io::{self, Write};
use std::path::PathBuf;
use tempfile::TempDir;

#[path = "coverage_gaps_test/delete_confirm.rs"]
mod delete_confirm;
#[path = "coverage_gaps_test/drive_selection.rs"]
mod drive_selection;
#[path = "coverage_gaps_test/event_loop.rs"]
mod event_loop;
#[path = "coverage_gaps_test/mcp_status.rs"]
mod mcp_status;
#[path = "coverage_gaps_test/virtual_path.rs"]
mod virtual_path;

const fn key_event(code: KeyCode) -> KeyEvent {
    KeyEvent::new(code, KeyModifiers::empty())
}

fn make_app(dir: &std::path::Path) -> io::Result<App> {
    let mut app = App::with_panels(
        Panel::new(dir.to_path_buf())?,
        Panel::new(dir.to_path_buf())?,
    );
    app.status = funny_file_manager::status::Status::Cleared;
    Ok(app)
}

fn make_terminal() -> io::Result<Terminal<WritableTestBackend>> {
    let backend = WritableTestBackend::new(240, 24);
    Terminal::new(backend)
}

fn run_events(app: App, events: Vec<Event>) -> io::Result<()> {
    let mut terminal = make_terminal()?;
    let mut iter = events.into_iter();
    run_app_with_event_reader(&mut terminal, app, || {
        iter.next()
            .ok_or_else(|| io::Error::new(io::ErrorKind::UnexpectedEof, "done"))
    })
}
