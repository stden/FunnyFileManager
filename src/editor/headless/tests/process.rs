//! Purpose: Exercise private editor subprocesses with isolated executable fixtures.
use super::*;
use std::os::unix::fs::PermissionsExt;

fn editor(directory: &Path, body: &str) -> std::path::PathBuf {
    let path = directory.join("private-editor.sh");
    fs::write(&path, format!("#!/bin/sh\n{body}\n")).unwrap();
    fs::set_permissions(&path, fs::Permissions::from_mode(0o755)).unwrap();
    path
}

#[test]
fn private_editor_receives_flags_and_absolute_selected_file() {
    let directory = tempfile::TempDir::new().unwrap();
    let path = directory.path().join("notes with spaces.txt");
    fs::write(&path, "before").unwrap();
    let command = editor(
        directory.path(),
        "test \"$1\" = --wait || exit 8; printf after > \"$2\"; printf EDITED",
    );
    let _editor = EditorEnvGuard::set(Path::new(&format!("{} --wait", command.display())));
    let mut app = build_app(&directory);
    assert!(select_entry(&mut app.left_panel, "notes with spaces.txt"));
    let host = HeadlessHost::new();
    let _scope = host.enter();
    edit(&mut app).unwrap();
    assert_eq!(fs::read_to_string(&path).unwrap(), "after");
    assert_eq!(app.status_text(), format!("Edited {}", path.display()));
    assert_eq!(app.console.visible(1)[0].text(), "EDITED");
    assert_eq!(host.snapshot().state, ExternalState::Exited { code: 0 });
}

#[test]
fn private_editor_nonzero_exit_keeps_the_transcript() {
    let directory = tempfile::TempDir::new().unwrap();
    fs::write(directory.path().join("notes.txt"), "before").unwrap();
    let command = editor(directory.path(), "printf 'could not save'; exit 6");
    let _editor = EditorEnvGuard::set(&command);
    let mut app = build_app(&directory);
    assert!(select_entry(&mut app.left_panel, "notes.txt"));
    let host = HeadlessHost::new();
    let _scope = host.enter();
    edit(&mut app).unwrap();
    assert_eq!(app.status_text(), "Editor exited with status exit code 6");
    assert_eq!(app.console.visible(1)[0].text(), "could not save");
}

#[test]
fn private_editor_does_not_swallow_inactive_panel_refresh_failure() {
    let directory = tempfile::TempDir::new().unwrap();
    fs::write(directory.path().join("notes.txt"), "before").unwrap();
    let command = editor(directory.path(), "exit 0");
    let _editor = EditorEnvGuard::set(&command);
    let mut app = build_app(&directory);
    assert!(select_entry(&mut app.left_panel, "notes.txt"));
    app.right_panel.path = "no-such-private-editor-panel".into();
    let host = HeadlessHost::new();
    let _scope = host.enter();
    assert_eq!(edit(&mut app).unwrap_err().kind(), io::ErrorKind::NotFound);
    assert!(app.status_text().starts_with("Edited "));
}
