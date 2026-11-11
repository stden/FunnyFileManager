//! Purpose: F8's confirmation: nothing selected, the parent row, and marks before the cursor.

use super::*;

// -----------------------------------------------------------------------
// handle_delete_confirm -- edge cases
// -----------------------------------------------------------------------

/// No selection (state = None) returns Ok without setting `delete_confirm`.
#[test]
fn test_handle_delete_confirm_no_selection() -> io::Result<()> {
    let mut app = App::new().expect("test setup");
    app.left_panel.select(None);

    handle_delete_confirm(&mut app).expect("should return Ok");

    assert!(!matches!(app.overlay, Overlay::DeleteConfirm(_)));

    Ok(())
}

/// Selecting ".." does NOT set `delete_confirm` (parent directory is protected).
#[test]
fn test_handle_delete_confirm_skips_parent_entry() -> io::Result<()> {
    let temp_dir = TempDir::new().expect("test setup");

    let mut app = App::new().expect("test setup");
    app.left_panel = Panel::new(temp_dir.path().to_path_buf()).expect("test setup");
    app.right_panel = Panel::new(temp_dir.path().to_path_buf()).expect("test setup");

    // ".." is always the first entry when a parent exists.
    let dotdot_index = app
        .left_panel
        .entries
        .iter()
        .position(|e| e.name == "..")
        .expect(".. entry must exist");

    app.left_panel.select(Some(dotdot_index));

    handle_delete_confirm(&mut app).expect("should return Ok");

    // delete_confirm must NOT be set -- we must not delete "..".
    assert!(!matches!(app.overlay, Overlay::DeleteConfirm(_)));

    Ok(())
}

/// Marked files take precedence over the current selection in `delete_confirm`.
#[test]
fn test_handle_delete_confirm_marked_takes_precedence() -> io::Result<()> {
    let temp_dir = TempDir::new().expect("test setup");
    let marked_file = temp_dir.path().join("marked.txt");
    let selected_file = temp_dir.path().join("selected.txt");
    File::create(&marked_file).expect("test setup");
    File::create(&selected_file).expect("test setup");

    let mut app = App::new().expect("test setup");
    app.left_panel = Panel::new(temp_dir.path().to_path_buf()).expect("test setup");
    app.right_panel = Panel::new(temp_dir.path().to_path_buf()).expect("test setup");

    // Select `selected_file` in the panel.
    let selected_index = app
        .left_panel
        .entries
        .iter()
        .position(|e| e.name == "selected.txt")
        .expect("selected.txt must be in entries");
    app.left_panel.select(Some(selected_index));

    // Mark `marked_file` but NOT `selected_file`.
    app.left_panel.marked.insert(FileName::from("marked.txt"));

    handle_delete_confirm(&mut app).expect("should return Ok");

    if let Overlay::DeleteConfirm(ref paths) = app.overlay {
        // The confirm list must contain the marked file, not the selected one.
        assert!(paths.iter().any(|p| p.ends_with("marked.txt")));
        assert!(!paths.iter().any(|p| p.ends_with("selected.txt")));
    } else {
        panic!("Expected DeleteConfirm overlay");
    }

    Ok(())
}
