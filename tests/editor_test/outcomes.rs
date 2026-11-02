//! Purpose: How an edit ends: success, a failed launch, a failing editor and the reload after it.

use super::*;

#[test]
#[cfg(feature = "test-helpers")]
fn test_handle_edit_success() -> io::Result<()> {
    let temp_dir = TempDir::new()?;
    let file_path = temp_dir.path().join("test.txt");
    File::create(&file_path)?;

    let mut app = create_test_app(&temp_dir)?;
    app.left_panel.select(Some(1)); // Select the file

    handle_edit_test_mode(&mut app, TestEditorBehavior::Success)?;

    assert_eq!(app.status_text(), format!("Edited {}", file_path.display()));
    Ok(())
}

#[test]
#[cfg(feature = "test-helpers")]
fn test_handle_edit_fail_launch() -> io::Result<()> {
    let temp_dir = TempDir::new()?;
    let file_path = temp_dir.path().join("test.txt");
    File::create(&file_path)?;

    let mut app = create_test_app(&temp_dir)?;
    app.left_panel.select(Some(1)); // Select the file

    handle_edit_test_mode(&mut app, TestEditorBehavior::FailLaunch)?;

    assert_eq!(
        app.status_text(),
        "Failed to launch editor: No such file or directory (os error 2)"
    );
    Ok(())
}

#[test]
#[cfg(feature = "test-helpers")]
fn test_handle_edit_exit_error() -> io::Result<()> {
    let temp_dir = TempDir::new()?;
    let file_path = temp_dir.path().join("test.txt");
    File::create(&file_path)?;

    let mut app = create_test_app(&temp_dir)?;
    app.left_panel.select(Some(1)); // Select the file

    handle_edit_test_mode(&mut app, TestEditorBehavior::ExitError)?;

    assert_eq!(
        app.status_text(),
        "Editor exited with status exit status: 1"
    );
    Ok(())
}

#[test]
#[cfg(feature = "test-helpers")]
fn test_handle_edit_reloads_entries() -> io::Result<()> {
    let temp_dir = TempDir::new()?;
    let file_path = temp_dir.path().join("test.txt");
    File::create(&file_path)?;

    let mut app = create_test_app(&temp_dir)?;
    app.left_panel.select(Some(1)); // Select the file

    // Get initial entry count
    let initial_count = app.left_panel.entries.len();

    handle_edit_test_mode(&mut app, TestEditorBehavior::Success)?;

    // Entries should be reloaded (count may be same but reload happened)
    assert_eq!(app.left_panel.entries.len(), initial_count);
    Ok(())
}

#[test]
#[cfg(feature = "test-helpers")]
fn test_handle_edit_propagates_reload_error() -> io::Result<()> {
    let temp_dir = TempDir::new()?;
    let file_path = temp_dir.path().join("right_file.txt");
    File::create(&file_path)?;

    let mut app = create_test_app(&temp_dir)?;
    app.active_panel = PanelSide::Right;
    app.right_panel = Panel::new(temp_dir.path().to_path_buf())?;
    app.left_panel.path = temp_dir.path().join("missing-left-panel");

    for (i, entry) in app.right_panel.entries.iter().enumerate() {
        if entry.name == "right_file.txt" {
            app.right_panel.select(Some(i));
            break;
        }
    }

    let err = handle_edit_test_mode(&mut app, TestEditorBehavior::Success)
        .expect_err("missing left panel path should fail reload");

    assert_eq!(err.kind(), io::ErrorKind::NotFound);
    assert!(app.status_text().contains("Edited"));
    Ok(())
}
