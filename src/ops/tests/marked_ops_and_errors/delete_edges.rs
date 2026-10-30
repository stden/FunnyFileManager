//! Purpose: F8 edge cases: no confirmation, partial errors, `..` and marked files.

use super::*;

// --- Delete edge cases ---

#[test]
fn test_handle_delete_no_confirm_set() -> io::Result<()> {
    let mut app = App::new().expect("test setup");
    // No longer exists: app.delete_confirm = None;

    // Should return Ok(()) without doing anything
    handle_delete(&mut app).expect("test setup");

    Ok(())
}

#[test]
fn test_handle_delete_with_partial_errors() -> io::Result<()> {
    let temp_dir = TempDir::new().expect("test setup");
    let existing = temp_dir.path().join("exists.txt");
    fs::write(&existing, b"data").expect("test setup");

    let nonexistent = temp_dir.path().join("gone.txt");

    let mut app = App::new().expect("test setup");
    app.left_panel = Panel::new(temp_dir.path().to_path_buf()).expect("test setup");
    app.right_panel = Panel::new(temp_dir.path().to_path_buf()).expect("test setup");
    app.set_overlay(Overlay::DeleteConfirm(vec![existing.clone(), nonexistent]));

    handle_delete(&mut app).expect("test setup");

    assert!(!existing.exists());
    // Should report both deleted count and errors
    assert!(app.status_text().contains("1 item(s)"));
    assert!(app.status_text().contains("error"));

    Ok(())
}

#[test]
fn test_handle_delete_confirm_dotdot_selected() -> io::Result<()> {
    let temp_dir = TempDir::new().expect("test setup");
    let subdir = temp_dir.path().join("sub");
    fs::create_dir(&subdir).expect("test setup");

    let mut app = App::new().expect("test setup");
    app.left_panel = Panel::new(subdir).expect("test setup");
    // Select ".." (should be index 0)
    app.left_panel.select(Some(0));

    handle_delete_confirm(&mut app).expect("test setup");
    // Should not set delete_confirm for ".."
    assert!(!matches!(app.overlay, Overlay::DeleteConfirm(_)));

    Ok(())
}

#[test]
fn test_handle_delete_confirm_marked_files() -> io::Result<()> {
    let temp_dir = TempDir::new().expect("test setup");
    fs::write(temp_dir.path().join("a.txt"), b"a").expect("test setup");
    fs::write(temp_dir.path().join("b.txt"), b"b").expect("test setup");

    let mut app = App::new().expect("test setup");
    app.left_panel = Panel::new(temp_dir.path().to_path_buf()).expect("test setup");
    app.right_panel = Panel::new(temp_dir.path().to_path_buf()).expect("test setup");

    app.left_panel.marked.insert(FileName::from("a.txt"));
    app.left_panel.marked.insert(FileName::from("b.txt"));

    handle_delete_confirm(&mut app).expect("test setup");

    if let Overlay::DeleteConfirm(paths) = &app.overlay {
        assert_eq!(paths.len(), 2);
    } else {
        panic!("Expected DeleteConfirm overlay");
    }

    Ok(())
}
