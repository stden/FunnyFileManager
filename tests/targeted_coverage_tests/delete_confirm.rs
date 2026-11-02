//! Purpose: Delete confirmation and `handle_delete` for `..`, marks and many files.

use super::*;

// ──────────────────────────────────────────────
// Delete confirm edge cases
// ──────────────────────────────────────────────

#[test]
fn test_delete_confirm_dotdot_is_skipped() -> io::Result<()> {
    let temp = TempDir::new()?;
    let subdir = temp.path().join("sub");
    fs::create_dir(&subdir)?;

    let mut app = App::new()?;
    app.left_panel.path = subdir;
    app.left_panel.load_entries()?;

    // Select ".." entry
    for (i, entry) in app.left_panel.entries.iter().enumerate() {
        if entry.name == ".." {
            app.left_panel.select(Some(i));
            break;
        }
    }

    handle_delete_confirm(&mut app)?;
    // Should NOT set delete_confirm for ".."
    assert!(app.get_delete_confirm().is_none());

    Ok(())
}

#[test]
fn test_delete_confirm_no_selection() -> io::Result<()> {
    let temp = TempDir::new()?;
    let mut app = App::new()?;
    app.left_panel.path = temp.path().to_path_buf();
    // Empty directory, no entries to select
    app.left_panel.entries.clear();
    app.left_panel.select(None);

    handle_delete_confirm(&mut app)?;
    assert!(app.get_delete_confirm().is_none());

    Ok(())
}

#[test]
fn test_delete_confirm_marked_files() -> io::Result<()> {
    let temp = TempDir::new()?;
    File::create(temp.path().join("a.txt"))?.write_all(b"a")?;
    File::create(temp.path().join("b.txt"))?.write_all(b"b")?;

    let mut app = App::new()?;
    app.left_panel.path = temp.path().to_path_buf();
    app.left_panel.load_entries()?;

    // Mark both files
    app.left_panel.marked.insert(FileName::from("a.txt"));
    app.left_panel.marked.insert(FileName::from("b.txt"));

    handle_delete_confirm(&mut app)?;
    assert!(app.get_delete_confirm().is_some());
    assert_eq!(app.get_delete_confirm().unwrap().len(), 2);

    Ok(())
}

#[test]
fn test_delete_confirm_overlay_yes_no() -> io::Result<()> {
    let temp = TempDir::new()?;
    File::create(temp.path().join("del.txt"))?.write_all(b"x")?;

    let mut app = App::new()?;
    app.left_panel.path = temp.path().to_path_buf();
    app.left_panel.load_entries()?;

    // Select del.txt
    for (i, entry) in app.left_panel.entries.iter().enumerate() {
        if entry.name == "del.txt" {
            app.left_panel.select(Some(i));
            break;
        }
    }

    // Set up delete confirm
    handle_delete_confirm(&mut app)?;
    assert!(app.get_delete_confirm().is_some());

    // Cancel with 'N'
    process_popup_input(&mut app, key(KeyCode::Char('n')));
    assert!(app.get_delete_confirm().is_none());
    assert!(app.status_text().contains("cancelled"));
    assert!(temp.path().join("del.txt").exists()); // File should still exist

    // Set up again and confirm with 'Y'
    handle_delete_confirm(&mut app)?;
    process_popup_input(&mut app, key(KeyCode::Char('y')));
    assert!(!temp.path().join("del.txt").exists()); // File should be deleted

    Ok(())
}

// ──────────────────────────────────────────────
// Search overlay
// ──────────────────────────────────────────────

// ──────────────────────────────────────────────
// Bookmark overlay
// ──────────────────────────────────────────────

// ──────────────────────────────────────────────
// handle_delete edge cases
// ──────────────────────────────────────────────

#[test]
fn test_handle_delete_no_confirm_pending() -> io::Result<()> {
    let mut app = App::new()?;
    app.close_overlay();

    let before = app.status_text();
    // Should be a no-op
    handle_delete(&mut app)?;
    assert_eq!(app.status_text(), before, "status should not change");

    Ok(())
}

#[test]
fn test_handle_delete_multiple_files() -> io::Result<()> {
    let temp = TempDir::new()?;
    File::create(temp.path().join("a.txt"))?.write_all(b"a")?;
    File::create(temp.path().join("b.txt"))?.write_all(b"b")?;
    let subdir = temp.path().join("mydir");
    fs::create_dir(&subdir)?;
    File::create(subdir.join("inner.txt"))?.write_all(b"in")?;

    let mut app = App::new()?;
    app.left_panel.path = temp.path().to_path_buf();
    app.left_panel.load_entries()?;

    // Manually set delete_confirm with multiple targets
    app.set_overlay(Overlay::DeleteConfirm(vec![
        temp.path().join("a.txt"),
        temp.path().join("b.txt"),
        temp.path().join("mydir"),
    ]));

    handle_delete(&mut app)?;
    assert!(!temp.path().join("a.txt").exists());
    assert!(!temp.path().join("b.txt").exists());
    assert!(!temp.path().join("mydir").exists());
    assert!(app.status_text().contains('3'));

    Ok(())
}
