//! Purpose: The delete confirmation: confirming, skipping `..`, errors and the status it leaves.

use super::*;

// -----------------------------------------------------------------------
// Delete confirmation overlay: Y confirms delete
// -----------------------------------------------------------------------

#[test]
fn test_delete_confirm_overlay_y_confirms() -> io::Result<()> {
    let temp_dir = TempDir::new()?;
    let file_path = temp_dir.path().join("to_delete.txt");
    fs::write(&file_path, "data")?;

    let mut app = App::new()?;
    app.left_panel = Panel::new(temp_dir.path().to_path_buf())?;
    app.right_panel = Panel::new(temp_dir.path().to_path_buf())?;
    app.set_overlay(Overlay::DeleteConfirm(vec![file_path.clone()]));

    process_popup_input(&mut app, key_event(KeyCode::Char('y')));
    assert!(!file_path.exists(), "file should be deleted");
    assert!(app.status_text().contains("Deleted"));
    Ok(())
}

// -----------------------------------------------------------------------
// handle_delete_confirm: ".." guard + no selection
// -----------------------------------------------------------------------

#[test]
fn test_delete_confirm_skips_dotdot() -> io::Result<()> {
    let temp_dir = TempDir::new()?;
    let sub = temp_dir.path().join("sub");
    fs::create_dir(&sub)?;

    let mut app = App::new()?;
    app.left_panel = Panel::new(sub)?;
    app.right_panel = Panel::new(temp_dir.path().to_path_buf())?;

    // Select ".."
    if let Some(pos) = app.left_panel.entries.iter().position(|e| e.name == "..") {
        app.left_panel.select(Some(pos));
    }

    handle_delete_confirm(&mut app)?;
    assert!(
        !matches!(app.overlay, Overlay::DeleteConfirm(_)),
        "should not confirm delete on .."
    );
    Ok(())
}

#[test]
fn test_delete_confirm_no_selection_is_noop() -> io::Result<()> {
    let temp_dir = TempDir::new()?;
    let mut app = App::new()?;
    app.left_panel = Panel::new(temp_dir.path().to_path_buf())?;
    app.right_panel = Panel::new(temp_dir.path().to_path_buf())?;
    app.left_panel.entries.clear();
    app.left_panel.select(None);

    handle_delete_confirm(&mut app)?;
    assert!(!matches!(app.overlay, Overlay::DeleteConfirm(_)));
    Ok(())
}

// -----------------------------------------------------------------------
// Delete confirmation overlay: Y confirms, error path
// -----------------------------------------------------------------------

#[test]
fn test_delete_confirm_overlay_error_path() -> io::Result<()> {
    let temp_dir = TempDir::new()?;
    let mut app = App::new()?;
    app.left_panel = Panel::new(temp_dir.path().to_path_buf())?;
    app.right_panel = Panel::new(temp_dir.path().to_path_buf())?;

    // Set up delete_confirm with a nonexistent path to trigger delete error
    app.set_overlay(Overlay::DeleteConfirm(vec![PathBuf::from(
        "/nonexistent_file_to_delete_99999",
    )]));

    process_popup_input(&mut app, key_event(KeyCode::Char('y')));
    assert!(
        app.status_text().contains("error"),
        "status: {}",
        app.status_text()
    );
    Ok(())
}

/// A panel that can no longer be listed is a *display* problem, not a delete
/// failure. This test used to assert the opposite — "Delete failed: No such file
/// or directory" for a delete that had actually happened — which is exactly the
/// bug: the user reads it, assumes the data is still there, and may retry.
#[test]
fn test_delete_confirm_overlay_keeps_success_status_when_refresh_fails() -> io::Result<()> {
    let temp_dir = TempDir::new()?;
    let file_path = temp_dir.path().join("deleteme.txt");
    fs::write(&file_path, "data")?;

    let mut app = App::new()?;
    app.left_panel = Panel::new(temp_dir.path().to_path_buf())?;
    app.right_panel = Panel::new(temp_dir.path().to_path_buf())?;
    // A panel path that cannot be read, so the post-delete refresh fails.
    app.left_panel.path = PathBuf::from("/nonexistent_dir_for_refresh_99999");
    app.set_overlay(Overlay::DeleteConfirm(vec![file_path.clone()]));

    process_popup_input(&mut app, key_event(KeyCode::Char('y')));

    // `symlink_metadata`, not `exists()`: the latter follows links.
    assert!(
        fs::symlink_metadata(&file_path).is_err(),
        "file was deleted"
    );
    assert_eq!(
        app.status_text(),
        "Deleted 1 item(s)",
        "a failed refresh must not be reported as a failed delete"
    );
    Ok(())
}

// -----------------------------------------------------------------------
// handle_delete_confirm: ".." selected returns early
// -----------------------------------------------------------------------

#[test]
fn test_handle_delete_confirm_dotdot_entry_selected() -> io::Result<()> {
    let temp_dir = TempDir::new()?;
    fs::write(temp_dir.path().join("keep.txt"), "data")?;

    let mut app = App::new()?;
    app.left_panel = Panel::new(temp_dir.path().to_path_buf())?;

    // Select ".." entry
    if let Some(idx) = app.left_panel.entries.iter().position(|e| e.name == "..") {
        app.left_panel.select(Some(idx));
    } else {
        // No ".." entry, test is not applicable at root
        return Ok(());
    }

    handle_delete_confirm(&mut app)?;

    // Should NOT set delete_confirm
    assert!(
        !matches!(app.overlay, Overlay::DeleteConfirm(_)),
        "should not confirm delete on .."
    );

    Ok(())
}
