//! Purpose: Cleanup regressions and outcomes using disposable filesystem fixtures.

use std::fs;

use tempfile::TempDir;

use crate::models::{FileSize, Overlay};
use crate::test_support::disk_cleanup::{app, candidate, dialog, heap_dumps, run};

mod deletion;
mod measurement;
mod temp_folder;

#[test]
fn cancelling_preserves_selected_files_and_closes_the_overlay() {
    let temp = TempDir::new().unwrap();
    let file = temp.path().join("keep.txt");
    fs::write(&file, b"keep").unwrap();
    let mut app = app(temp.path(), vec![candidate("Cache file", &file)]);

    super::cancel(&mut app);

    assert!(matches!(app.overlay, Overlay::None));
    assert_eq!(app.status.to_string(), "Cleanup cancelled");
    assert_eq!(fs::read(file).unwrap(), b"keep");
}

#[test]
fn running_without_a_cleanup_overlay_preserves_state() {
    let temp = TempDir::new().unwrap();
    let mut app = app(temp.path(), Vec::new());
    app.set_overlay(Overlay::Help);
    let before = app.status.to_string();

    run(&mut app);

    assert!(matches!(app.overlay, Overlay::Help));
    assert_eq!(app.status.to_string(), before);
    assert_eq!(super::current_field(&app), None);
}

#[test]
fn an_empty_selection_closes_without_deleting_files() {
    let temp = TempDir::new().unwrap();
    let file = temp.path().join("keep.txt");
    fs::write(&file, b"keep").unwrap();
    let mut row = candidate("Cache file", &file);
    row.checked = false;
    let mut app = app(temp.path(), vec![row]);

    run(&mut app);

    assert!(matches!(app.overlay, Overlay::None));
    assert_eq!(app.status.to_string(), "Nothing selected for cleanup");
    assert_eq!(fs::read(file).unwrap(), b"keep");
}
