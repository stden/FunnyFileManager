//! Purpose: Adds extra integration coverage for file operations and popup handling.
//!

use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};
use funny_file_manager::handlers::{
    handle_copy, handle_delete_confirm, handle_mkdir, handle_move, process_popup_input,
};
use funny_file_manager::models::FileName;
use funny_file_manager::models::{App, Overlay, Panel, PanelSide, SortMode};
use std::fs::{self, File};
use std::io;
use std::path::PathBuf;
use tempfile::TempDir;

#[path = "extra_coverage_test/commands.rs"]
mod commands;
#[path = "extra_coverage_test/event_loop.rs"]
mod event_loop;
#[path = "extra_coverage_test/file_handlers.rs"]
mod file_handlers;
#[path = "extra_coverage_test/panel_state.rs"]
mod panel_state;
#[path = "extra_coverage_test/popup_input.rs"]
mod popup_input;

const fn key_event(code: KeyCode) -> KeyEvent {
    KeyEvent::new(code, KeyModifiers::empty())
}
