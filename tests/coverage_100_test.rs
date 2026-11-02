//! Purpose: Targets coverage gaps across popup input, panels, and utility behavior.
//!

use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};
use funny_file_manager::handlers::process_popup_input;
use funny_file_manager::models::EntryKind;
use funny_file_manager::models::FileName;
use funny_file_manager::models::FileSize;
use funny_file_manager::models::Modified;
use funny_file_manager::models::{App, DirEntry, Overlay, Panel, SortMode};
use funny_file_manager::utils::{copy_dir_all, refresh_panels, unique_destination};
use std::{fs, io};
use tempfile::TempDir;

#[path = "coverage_100_test/copy_and_refresh.rs"]
mod copy_and_refresh;
#[path = "coverage_100_test/event_loop.rs"]
mod event_loop;
#[path = "coverage_100_test/names_and_sizes.rs"]
mod names_and_sizes;
#[path = "coverage_100_test/panel_state.rs"]
mod panel_state;
#[path = "coverage_100_test/popup_input.rs"]
mod popup_input;

const fn key_event(code: KeyCode, modifiers: KeyModifiers) -> KeyEvent {
    KeyEvent::new(code, modifiers)
}
