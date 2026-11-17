//! Purpose: Strengthens handler assertions against mutation-test survivors.
//!

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use funny_file_manager::handlers::{
    handle_copy, handle_delete, handle_delete_confirm, handle_mkdir, handle_move, handle_view,
    process_popup_input,
};
use funny_file_manager::models::FileName;
use funny_file_manager::models::{App, Overlay, Viewer};
use funny_file_manager::scroll::ScrollTop;
use funny_file_manager::ui::ui;
use ratatui::backend::TestBackend;
use ratatui::Terminal;
use std::fs::{self, File};
use std::io::{self, Write};
use tempfile::TempDir;

#[path = "handlers_mutation_tests/mkdir_and_delete.rs"]
mod mkdir_and_delete;
#[path = "handlers_mutation_tests/popup_input.rs"]
mod popup_input;
#[path = "handlers_mutation_tests/view_and_transfer.rs"]
mod view_and_transfer;
