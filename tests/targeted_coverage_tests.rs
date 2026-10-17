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

#[path = "targeted_coverage_tests/command_line_builtins.rs"]
mod command_line_builtins;
#[path = "targeted_coverage_tests/command_line_navigation.rs"]
mod command_line_navigation;
#[path = "targeted_coverage_tests/command_line_shell.rs"]
mod command_line_shell;
#[path = "targeted_coverage_tests/delete_confirm.rs"]
mod delete_confirm;
#[path = "targeted_coverage_tests/drive_selection.rs"]
mod drive_selection;
#[path = "targeted_coverage_tests/event_loop_file_keys.rs"]
mod event_loop_file_keys;
#[path = "targeted_coverage_tests/event_loop_marks_search.rs"]
mod event_loop_marks_search;
#[path = "targeted_coverage_tests/event_loop_navigation.rs"]
mod event_loop_navigation;
#[path = "targeted_coverage_tests/event_loop_overlays.rs"]
mod event_loop_overlays;
#[path = "targeted_coverage_tests/file_operations.rs"]
mod file_operations;
#[path = "targeted_coverage_tests/new_file_input.rs"]
mod new_file_input;
#[path = "targeted_coverage_tests/panel_state.rs"]
mod panel_state;
#[path = "targeted_coverage_tests/viewer.rs"]
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
