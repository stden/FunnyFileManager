//! Purpose: Tests of `handle_edit_test_mode`, the explicit integration-test seam.
use super::*;

#[test]
fn test_handle_edit_test_mode_is_safe() -> io::Result<()> {
    let temp_dir = TempDir::new().expect("temp dir");
    let file_path = temp_dir.path().join("file.txt");
    fs::write(&file_path, b"content").expect("write file");

    let mut app = build_app(&temp_dir);
    assert!(select_entry(&mut app.left_panel, "file.txt"));

    let result = handle_edit_test_mode(&mut app, TestEditorBehavior::Success);

    assert!(result.is_ok());
    assert!(app.status_text().contains("Edited"));
    Ok(())
}

/// No selection: `handle_edit_test_mode` returns early without touching the
/// status line, mirroring `handle_edit_impl`'s own no-selection guard.
#[test]
fn test_handle_edit_test_mode_no_selection() -> io::Result<()> {
    let temp_dir = TempDir::new().expect("temp dir");
    let mut app = build_app(&temp_dir);
    app.left_panel.select(None);

    let result = handle_edit_test_mode(&mut app, TestEditorBehavior::Success);

    assert!(result.is_ok());
    assert!(app.status_text().is_empty());
    Ok(())
}

/// A directory selection is rejected before any behavior is simulated.
#[test]
fn test_handle_edit_test_mode_directory_selected() -> io::Result<()> {
    let temp_dir = TempDir::new().expect("temp dir");
    let subdir = temp_dir.path().join("subdir");
    fs::create_dir(&subdir).expect("create subdir");

    let mut app = build_app(&temp_dir);
    assert!(select_entry(&mut app.left_panel, "subdir"));

    let result = handle_edit_test_mode(&mut app, TestEditorBehavior::Success);

    assert!(result.is_ok());
    assert_eq!(app.status_text(), "Editing directories is not supported");
    Ok(())
}

/// `TestEditorBehavior::FailLaunch` reports the same status text a real
/// launch failure would produce.
#[test]
fn test_handle_edit_test_mode_fail_launch() -> io::Result<()> {
    let temp_dir = TempDir::new().expect("temp dir");
    let file_path = temp_dir.path().join("file.txt");
    fs::write(&file_path, b"content").expect("write file");

    let mut app = build_app(&temp_dir);
    assert!(select_entry(&mut app.left_panel, "file.txt"));

    let result = handle_edit_test_mode(&mut app, TestEditorBehavior::FailLaunch);

    assert!(result.is_ok());
    assert_eq!(
        app.status_text(),
        "Failed to launch editor: No such file or directory (os error 2)"
    );
    Ok(())
}

/// `TestEditorBehavior::ExitError` reports the same status text a
/// non-zero editor exit would produce.
#[test]
fn test_handle_edit_test_mode_exit_error() -> io::Result<()> {
    let temp_dir = TempDir::new().expect("temp dir");
    let file_path = temp_dir.path().join("file.txt");
    fs::write(&file_path, b"content").expect("write file");

    let mut app = build_app(&temp_dir);
    assert!(select_entry(&mut app.left_panel, "file.txt"));

    let result = handle_edit_test_mode(&mut app, TestEditorBehavior::ExitError);

    assert!(result.is_ok());
    assert_eq!(
        app.status_text(),
        "Editor exited with status exit status: 1"
    );
    Ok(())
}
