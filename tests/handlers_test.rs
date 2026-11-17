//! Purpose: Covers handler workflows at the integration-test level.
//!

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use funny_file_manager::handlers::{
    handle_copy, handle_delete, handle_delete_confirm, handle_mkdir, handle_move, handle_view,
    process_popup_input,
};
use funny_file_manager::models::FileName;
use funny_file_manager::models::{App, Overlay, Panel, PanelSide, SortMode, Viewer};
use funny_file_manager::scroll::ScrollTop;
use std::fs::{self, File};
use std::io::{self, Write};
use tempfile::TempDir;

#[path = "handlers_test/command_line.rs"]
mod command_line;
#[path = "handlers_test/command_line_shell.rs"]
mod command_line_shell;
#[path = "handlers_test/file_handlers.rs"]
mod file_handlers;
#[path = "handlers_test/popup_input.rs"]
mod popup_input;

const fn key_event(code: KeyCode) -> KeyEvent {
    KeyEvent::new(code, KeyModifiers::empty())
}
