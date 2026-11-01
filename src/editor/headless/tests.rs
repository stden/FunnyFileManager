//! Purpose: Private editor requests retain guard, error and transcript behavior.
use super::*;
use crate::editor::tests::{build_app, select_entry};
use crate::terminal::headless::{ExternalOutcome, ExternalState, HeadlessHost};
use crate::test_support::EditorEnvGuard;
use std::{fs, path::Path};

#[test]
fn absent_selection_and_directories_do_not_start_a_program() {
    let directory = tempfile::TempDir::new().unwrap();
    fs::create_dir(directory.path().join("folder")).unwrap();
    let mut app = build_app(&directory);
    let host = HeadlessHost::new();
    let _scope = host.enter();
    app.left_panel.select(None);
    edit(&mut app).unwrap();
    assert_eq!(app.status_text(), "");
    assert!(select_entry(&mut app.left_panel, "folder"));
    edit(&mut app).unwrap();
    assert_eq!(app.status_text(), "Editing directories is not supported");
    assert_eq!(host.snapshot().state, ExternalState::Idle);
}

#[test]
fn missing_editor_reports_launch_failure_and_refreshes_panels() {
    let directory = tempfile::TempDir::new().unwrap();
    fs::write(directory.path().join("notes.txt"), "before").unwrap();
    let mut app = build_app(&directory);
    assert!(select_entry(&mut app.left_panel, "notes.txt"));
    let _editor = EditorEnvGuard::set(&directory.path().join("missing-editor"));
    let host = HeadlessHost::new();
    let _scope = host.enter();
    fs::write(directory.path().join("new.txt"), "new").unwrap();
    edit(&mut app).unwrap();
    assert!(app.status_text().starts_with("Failed to launch editor:"));
    assert_eq!(host.snapshot().state, ExternalState::Failed);
    assert!(app
        .right_panel
        .entries
        .iter()
        .any(|entry| entry.name == "new.txt"));
}

#[test]
fn editor_outcomes_record_lossy_output_and_preserve_failure_status() {
    let directory = tempfile::TempDir::new().unwrap();
    let mut app = build_app(&directory);
    let path = Path::new("notes.txt");
    let success = ExternalOutcome {
        code: 0,
        output: b"editor output\n".to_vec(),
    };
    assert!(
        matches!(outcome(&mut app, Ok(success), path), Status::Edited(name) if name == "notes.txt")
    );
    let failure = ExternalOutcome {
        code: 9,
        output: vec![b'x', 0xff],
    };
    assert!(
        matches!(outcome(&mut app, Ok(failure), path), Status::EditorExited { status } if status == "exit code 9")
    );
    assert_eq!(app.console.len(), 2);
    assert_eq!(app.console.visible(2)[1].text(), "x�");
    let error = io::Error::new(io::ErrorKind::PermissionDenied, "editor denied");
    assert!(
        matches!(outcome(&mut app, Err(error), path), Status::LaunchEditorFailed { error } if error == "editor denied")
    );
}

#[test]
fn default_editor_cannot_escape_the_missing_private_host() {
    let directory = tempfile::TempDir::new().unwrap();
    fs::write(directory.path().join("notes.txt"), "before").unwrap();
    let mut app = build_app(&directory);
    assert!(select_entry(&mut app.left_panel, "notes.txt"));
    let _editor = EditorEnvGuard::clear();
    edit(&mut app).unwrap();
    assert!(app.status_text().starts_with("Failed to launch editor:"));
    assert!(app.console.is_empty());
}

#[cfg(unix)]
mod process;
