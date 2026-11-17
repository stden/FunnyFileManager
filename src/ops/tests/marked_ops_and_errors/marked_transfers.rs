//! Purpose: Copy and move of several marked files, and the directory rename fallback.

use super::*;

#[test]
fn test_handle_copy_no_selection_logic() -> io::Result<()> {
    let mut app = App::new().expect("test setup");
    app.left_panel.entries.clear();
    app.left_panel.select(None);

    // Should return Ok(()) and do nothing
    handle_copy(&mut app).expect("test setup");

    Ok(())
}

// --- Multi-file marked operations ---

#[test]
fn test_handle_copy_multiple_marked_files() -> io::Result<()> {
    let temp_dir = TempDir::new().expect("test setup");
    let left_dir = temp_dir.path().join("left");
    let right_dir = temp_dir.path().join("right");
    fs::create_dir(&left_dir).expect("test setup");
    fs::create_dir(&right_dir).expect("test setup");

    fs::write(left_dir.join("a.txt"), b"aaa").expect("test setup");
    fs::write(left_dir.join("b.txt"), b"bbb").expect("test setup");
    fs::write(left_dir.join("c.txt"), b"ccc").expect("test setup");

    let mut app = App::new().expect("test setup");
    app.left_panel = Panel::new(left_dir).expect("test setup");
    app.right_panel = Panel::new(right_dir.clone()).expect("test setup");

    // Mark multiple files
    app.left_panel.marked.insert(FileName::from("a.txt"));
    app.left_panel.marked.insert(FileName::from("b.txt"));

    handle_copy(&mut app).expect("test setup");

    assert!(right_dir.join("a.txt").exists());
    assert!(right_dir.join("b.txt").exists());
    // More than one entry: counted, not named.
    assert_eq!(app.status_text(), "Copied 2 item(s)");

    Ok(())
}

#[test]
fn test_handle_move_multiple_marked_files() -> io::Result<()> {
    let temp_dir = TempDir::new().expect("test setup");
    let left_dir = temp_dir.path().join("left");
    let right_dir = temp_dir.path().join("right");
    fs::create_dir(&left_dir).expect("test setup");
    fs::create_dir(&right_dir).expect("test setup");

    fs::write(left_dir.join("x.txt"), b"xxx").expect("test setup");
    fs::write(left_dir.join("y.txt"), b"yyy").expect("test setup");

    let mut app = App::new().expect("test setup");
    app.left_panel = Panel::new(left_dir.clone()).expect("test setup");
    app.right_panel = Panel::new(right_dir.clone()).expect("test setup");

    app.left_panel.marked.insert(FileName::from("x.txt"));
    app.left_panel.marked.insert(FileName::from("y.txt"));

    handle_move(&mut app).expect("test setup");

    assert!(!left_dir.join("x.txt").exists());
    assert!(!left_dir.join("y.txt").exists());
    assert!(right_dir.join("x.txt").exists());
    assert!(right_dir.join("y.txt").exists());
    assert!(app.status_text().contains("Moved"));
    assert!(app.status_text().contains('2'));

    Ok(())
}

// --- Move with forced rename failure (directory fallback) ---

#[test]
fn test_handle_move_dir_fallback_on_rename_failure() -> io::Result<()> {
    let temp_dir = TempDir::new().expect("test setup");
    let left_dir = temp_dir.path().join("left");
    let right_dir = temp_dir.path().join("right");
    fs::create_dir(&left_dir).expect("test setup");
    fs::create_dir(&right_dir).expect("test setup");

    let source_dir = left_dir.join("mydir");
    fs::create_dir(&source_dir).expect("test setup");
    fs::write(source_dir.join("inner.txt"), b"data").expect("test setup");

    let mut app = App::new().expect("test setup");
    app.left_panel = Panel::new(left_dir).expect("test setup");
    app.right_panel = Panel::new(right_dir.clone()).expect("test setup");

    for (i, entry) in app.left_panel.entries.iter().enumerate() {
        if entry.name == "mydir" {
            app.left_panel.select(Some(i));
            break;
        }
    }

    let _guard = ForceRenameFailureGuard::enable();
    handle_move(&mut app).expect("test setup");

    // Should have been copy+delete
    assert!(!source_dir.exists());
    assert!(right_dir.join("mydir").exists());
    assert!(right_dir.join("mydir").join("inner.txt").exists());

    Ok(())
}
