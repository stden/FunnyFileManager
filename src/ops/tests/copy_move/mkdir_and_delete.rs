//! Purpose: F7 more than once, and F8 with nothing selected.

use super::*;

#[test]
fn test_handle_mkdir_multiple() -> io::Result<()> {
    let temp_dir = TempDir::new().expect("test setup");

    let mut app = App::new().expect("test setup");
    app.left_panel = Panel::new(temp_dir.path().to_path_buf()).expect("test setup");
    app.right_panel = Panel::new(temp_dir.path().to_path_buf()).expect("test setup");

    // Create first directory
    handle_mkdir(&mut app).expect("test setup");
    assert!(temp_dir.path().join("new_folder").exists());

    // Create second directory (should be numbered)
    handle_mkdir(&mut app).expect("test setup");
    assert!(temp_dir.path().join("new_folder_1").exists());

    // Create third directory
    handle_mkdir(&mut app).expect("test setup");
    assert!(temp_dir.path().join("new_folder_2").exists());

    Ok(())
}

#[test]
fn test_handle_delete_no_selection() -> io::Result<()> {
    let mut app = App::new().expect("test setup");
    app.left_panel.select(None);

    // Should not crash
    handle_delete(&mut app).expect("test setup");

    Ok(())
}
