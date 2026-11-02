//! Purpose: Unit tests for host-association open seam, empty selection, and headless error reporting.

use super::{selected, take_opened};
use crate::models::{App, Panel, TerminalMode};
use crate::status::Status;

#[test]
fn selected_handles_interactive_seam_headless_failure_and_empty_panel() {
    let temp = tempfile::TempDir::new().unwrap();
    std::fs::write(temp.path().join("doc.txt"), b"hi").unwrap();
    let left = Panel::new(temp.path().to_path_buf()).unwrap();
    let right = Panel::new(temp.path().to_path_buf()).unwrap();
    let mut app = App::with_panels(left, right);
    assert!(app.left_panel.select_named("doc.txt"));
    drop(take_opened());
    selected(&mut app).unwrap();
    assert!(take_opened().is_some());
    assert_eq!(app.status, Status::Cleared);

    app.live.terminal_mode = TerminalMode::Headless;
    selected(&mut app).unwrap();
    assert!(matches!(app.status, Status::CannotRun { .. }));

    app.left_panel = Panel::from_entries(temp.path().to_owned(), Vec::new());
    selected(&mut app).unwrap();
}
