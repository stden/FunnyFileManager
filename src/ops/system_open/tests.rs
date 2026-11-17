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

#[test]
fn an_empty_selection_preserves_the_previous_status() {
    let root = tempfile::TempDir::new().unwrap();
    let panel = Panel::from_entries(root.path().to_owned(), Vec::new());
    let mut app = App::with_panels(panel.clone(), panel);
    app.set_status(Status::CommandOutput("previous outcome".to_owned()));
    drop(take_opened());
    selected(&mut app).unwrap();
    assert_eq!(
        app.status,
        Status::CommandOutput("previous outcome".to_owned())
    );
    assert!(take_opened().is_none());
}

#[test]
fn a_missing_private_host_reports_the_document_that_could_not_open() {
    let root = tempfile::TempDir::new().unwrap();
    let document = root.path().join("document.txt");
    std::fs::write(&document, b"keep").unwrap();
    let panel = Panel::new(root.path().to_owned()).unwrap();
    let mut app = App::with_panels(panel.clone(), panel);
    assert!(app.left_panel.select_named("document.txt"));
    app.live.terminal_mode = TerminalMode::Headless;
    drop(take_opened());
    selected(&mut app).unwrap();
    assert!(matches!(
        &app.status,
        Status::CannotRun { program, error }
            if program == &document && error.contains("No private terminal")
    ));
    assert_eq!(std::fs::read(&document).unwrap(), b"keep");
    assert!(take_opened().is_none());
}
