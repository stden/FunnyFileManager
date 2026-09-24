//! Purpose: Shift+F4 onto an existing or uncreatable name, and F3 on a file too large to view.

use super::*;

// -----------------------------------------------------------------------
// New file input: file already exists + create error
// -----------------------------------------------------------------------

#[test]
fn test_new_file_input_file_already_exists() -> io::Result<()> {
    let temp_dir = TempDir::new()?;
    fs::write(temp_dir.path().join("exists.txt"), "data")?;

    let mut app = App::new()?;
    app.left_panel = Panel::new(temp_dir.path().to_path_buf())?;
    app.right_panel = Panel::new(temp_dir.path().to_path_buf())?;
    app.set_overlay(Overlay::NewFileInput("exists.txt".to_owned()));

    process_popup_input(&mut app, key_event(KeyCode::Enter));
    assert!(
        app.status_text().contains("already exists"),
        "status: {}",
        app.status_text()
    );
    Ok(())
}

#[test]
fn test_new_file_create_error_sets_status() -> io::Result<()> {
    let temp_dir = TempDir::new()?;
    let mut app = App::new()?;
    app.left_panel = Panel::new(temp_dir.path().to_path_buf())?;
    app.right_panel = Panel::new(temp_dir.path().to_path_buf())?;
    // Try to create a file in a nonexistent subdirectory
    app.set_overlay(Overlay::NewFileInput("nonexistent_dir/file.txt".to_owned()));

    process_popup_input(&mut app, key_event(KeyCode::Enter));
    assert!(
        app.status_text().contains("Failed"),
        "status: {}",
        app.status_text()
    );
    Ok(())
}

// -----------------------------------------------------------------------
// File too large to view
// -----------------------------------------------------------------------

#[test]
fn test_handle_view_file_too_large() -> io::Result<()> {
    let temp_dir = TempDir::new()?;
    let big_file = temp_dir.path().join("big.bin");

    // Create a sparse file > 10 MB by writing just past the limit
    {
        let f = File::create(&big_file)?;
        f.set_len(11 * 1024 * 1024)?; // 11 MB
    }

    let mut app = App::new()?;
    app.left_panel = Panel::new(temp_dir.path().to_path_buf())?;
    if let Some(idx) = app
        .left_panel
        .entries
        .iter()
        .position(|e| e.name == "big.bin")
    {
        app.left_panel.select(Some(idx));
    }

    handle_view(&mut app)?;

    assert!(
        app.get_viewer().is_none(),
        "viewer should not open for large file"
    );
    assert!(
        app.status_text().contains("too large"),
        "status: {}",
        app.status_text()
    );

    Ok(())
}
