//! Purpose: Deletes that partly fail, or have nothing confirmed.

use super::*;

// -----------------------------------------------------------------------
// handle_delete with partial errors
// -----------------------------------------------------------------------

#[test]
fn test_handle_delete_with_nonexistent_file() -> io::Result<()> {
    let temp_dir = TempDir::new()?;
    let mut app = App::new()?;
    app.left_panel = Panel::new(temp_dir.path().to_path_buf())?;

    // Set up delete_confirm with a file that doesn't exist
    app.set_overlay(Overlay::DeleteConfirm(vec![temp_dir
        .path()
        .join("ghost.txt")]));
    handle_delete(&mut app)?;

    assert!(
        app.status_text().contains("error"),
        "should report error for nonexistent file: {}",
        app.status_text()
    );

    Ok(())
}

#[test]
fn test_handle_delete_mixed_success_and_error() -> io::Result<()> {
    let temp_dir = TempDir::new()?;
    let real_file = temp_dir.path().join("real.txt");
    fs::write(&real_file, "data")?;

    let mut app = App::new()?;
    app.left_panel = Panel::new(temp_dir.path().to_path_buf())?;

    // One real file + one nonexistent
    app.set_overlay(Overlay::DeleteConfirm(vec![
        real_file.clone(),
        temp_dir.path().join("ghost.txt"),
    ]));
    handle_delete(&mut app)?;

    assert!(!real_file.exists(), "real file should be deleted");
    assert!(
        app.status_text().contains("1 error"),
        "should report partial error: {}",
        app.status_text()
    );
    assert!(
        app.status_text().contains("Deleted 1"),
        "should report 1 deleted: {}",
        app.status_text()
    );

    Ok(())
}

#[test]
fn test_handle_delete_no_confirm_is_noop() -> io::Result<()> {
    let mut app = App::new()?;
    app.close_overlay();

    handle_delete(&mut app)?;
    // Should just return Ok without doing anything

    Ok(())
}
