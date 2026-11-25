//! Targeted tests to cover remaining uncovered lines and important edge cases.
//! Focus areas: new file input overlay, delete confirm edge cases, shell commands,
//! multi-file operations, viewer edge cases, event loop paths, `format_operation_status`.

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use funny_file_manager::handlers::{
    handle_copy, handle_delete, handle_delete_confirm, handle_mkdir, handle_move, handle_view,
    process_popup_input,
};
use funny_file_manager::models::FileName;
use funny_file_manager::models::{App, Overlay, PanelSide, Viewer};
use std::fs::{self, File};
use std::io::{self, Write};
use tempfile::TempDir;

mod command_line_builtins;
mod command_line_navigation;
mod command_line_shell;
mod delete_confirm;
mod drive_selection;
mod event_loop_file_keys;
mod event_loop_marks_search;
mod event_loop_navigation;
mod event_loop_overlays;
mod file_operations;
mod new_file_input;
mod panel_state;
mod viewer;

const fn key(code: KeyCode) -> KeyEvent {
    KeyEvent::new(code, KeyModifiers::empty())
}

const fn key_ctrl(code: KeyCode) -> KeyEvent {
    KeyEvent::new(code, KeyModifiers::CONTROL)
}

#[cfg(windows)]
fn shell_stderr_command() -> &'static str {
    "echo error 1>&2"
}

#[cfg(not(windows))]
const fn shell_stderr_command() -> &'static str {
    "echo error >&2"
}

#[cfg(windows)]
fn shell_failure_command() -> &'static str {
    "cmd /c exit /b 1"
}

#[cfg(not(windows))]
const fn shell_failure_command() -> &'static str {
    "false"
}

#[cfg(windows)]
fn shell_success_no_output_command() -> &'static str {
    "ver >nul"
}

#[cfg(not(windows))]
const fn shell_success_no_output_command() -> &'static str {
    "true"
}

// ──────────────────────────────────────────────
// Event loop (lib.rs) paths — run_app_with_event_reader takes App by value,
// so we can only verify it completes without panic.
// ──────────────────────────────────────────────

fn run_event_sequence(app: App, events: Vec<crossterm::event::Event>) -> io::Result<()> {
    use funny_file_manager::run_app_with_event_reader;
    use funny_file_manager::WritableTestBackend;
    use ratatui::Terminal;

    let backend = WritableTestBackend::new(120, 40);
    let mut terminal = Terminal::new(backend)?;

    let mut events = events.into_iter();
    run_app_with_event_reader(&mut terminal, app, || {
        Ok(events
            .next()
            .unwrap_or_else(|| crossterm::event::Event::Key(key(KeyCode::F(10)))))
    })
}
