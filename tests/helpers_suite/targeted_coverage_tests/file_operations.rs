//! Purpose: Copy, move and make-folder on marked files, directories and no selection.

use super::*;

// ──────────────────────────────────────────────
// Multi-file copy/move/delete
// ──────────────────────────────────────────────

#[test]
fn test_copy_multiple_marked_files() -> io::Result<()> {
    let temp = TempDir::new()?;
    let left = temp.path().join("left");
    let right = temp.path().join("right");
    fs::create_dir(&left)?;
    fs::create_dir(&right)?;
    File::create(left.join("a.txt"))?.write_all(b"aaa")?;
    File::create(left.join("b.txt"))?.write_all(b"bbb")?;
    File::create(left.join("c.txt"))?.write_all(b"ccc")?;

    let mut app = App::new()?;
    app.left_panel.path = left;
    app.right_panel.path = right.clone();
    app.left_panel.load_entries()?;
    app.right_panel.load_entries()?;

    // Mark multiple files
    app.left_panel.marked.insert(FileName::from("a.txt"));
    app.left_panel.marked.insert(FileName::from("b.txt"));
    app.left_panel.marked.insert(FileName::from("c.txt"));

    handle_copy(&mut app)?;

    assert!(right.join("a.txt").exists());
    assert!(right.join("b.txt").exists());
    assert!(right.join("c.txt").exists());
    // Multi-file status uses count format
    assert!(
        app.status_text().contains('3'),
        "Got: {}",
        app.status_text()
    );

    Ok(())
}

#[test]
fn test_move_multiple_marked_files() -> io::Result<()> {
    let temp = TempDir::new()?;
    let left = temp.path().join("left");
    let right = temp.path().join("right");
    fs::create_dir(&left)?;
    fs::create_dir(&right)?;
    File::create(left.join("x.txt"))?.write_all(b"x")?;
    File::create(left.join("y.txt"))?.write_all(b"y")?;

    let mut app = App::new()?;
    app.left_panel.path = left.clone();
    app.right_panel.path = right.clone();
    app.left_panel.load_entries()?;
    app.right_panel.load_entries()?;

    app.left_panel.marked.insert(FileName::from("x.txt"));
    app.left_panel.marked.insert(FileName::from("y.txt"));

    handle_move(&mut app)?;

    assert!(!left.join("x.txt").exists());
    assert!(!left.join("y.txt").exists());
    assert!(right.join("x.txt").exists());
    assert!(right.join("y.txt").exists());
    assert!(
        app.status_text().contains('2'),
        "Got: {}",
        app.status_text()
    );

    Ok(())
}

#[test]
fn test_copy_directory() -> io::Result<()> {
    let temp = TempDir::new()?;
    let left = temp.path().join("left");
    let right = temp.path().join("right");
    fs::create_dir(&left)?;
    fs::create_dir(&right)?;

    // Create a subdirectory with content
    let subdir = left.join("mydir");
    fs::create_dir(&subdir)?;
    File::create(subdir.join("inner.txt"))?.write_all(b"inner")?;

    let mut app = App::new()?;
    app.left_panel.path = left;
    app.right_panel.path = right.clone();
    app.left_panel.load_entries()?;
    app.right_panel.load_entries()?;

    // Select the directory
    for (i, entry) in app.left_panel.entries.iter().enumerate() {
        if entry.name == "mydir" {
            app.left_panel.select(Some(i));
            break;
        }
    }

    handle_copy(&mut app)?;
    assert!(right.join("mydir").is_dir());
    assert!(right.join("mydir").join("inner.txt").exists());

    Ok(())
}

#[test]
fn test_copy_no_selection() -> io::Result<()> {
    let temp = TempDir::new()?;
    let mut app = App::new()?;
    app.left_panel.path = temp.path().to_path_buf();
    app.left_panel.entries.clear();
    app.left_panel.select(None);

    // Should return Ok without error
    handle_copy(&mut app)?;
    assert!(app.status_text().is_empty() || !app.status_text().contains("Copied"));

    Ok(())
}

#[test]
fn test_move_no_selection() -> io::Result<()> {
    let temp = TempDir::new()?;
    let mut app = App::new()?;
    app.left_panel.path = temp.path().to_path_buf();
    app.left_panel.entries.clear();
    app.left_panel.select(None);

    handle_move(&mut app)?;
    assert!(app.status_text().is_empty() || !app.status_text().contains("Moved"));

    Ok(())
}

// ──────────────────────────────────────────────
// handle_mkdir edge case
// ──────────────────────────────────────────────

#[test]
fn test_mkdir_auto_increments_name() -> io::Result<()> {
    let temp = TempDir::new()?;
    fs::create_dir(temp.path().join("new_folder"))?;
    fs::create_dir(temp.path().join("new_folder_1"))?;

    let mut app = App::new()?;
    app.left_panel.path = temp.path().to_path_buf();
    app.left_panel.load_entries()?;

    handle_mkdir(&mut app)?;
    assert!(temp.path().join("new_folder_2").is_dir());

    Ok(())
}
