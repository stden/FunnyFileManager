//! Purpose: The view, copy and move handlers.

use super::*;

#[test]
fn test_handle_view_with_file() -> io::Result<()> {
    let temp_dir = TempDir::new()?;
    let mut app = App::new()?;

    // Create test file
    let file_path = temp_dir.path().join("test.txt");
    File::create(&file_path)?.write_all(b"Test content\nLine 2\nLine 3")?;

    app.left_panel.path = temp_dir.path().to_path_buf();
    app.left_panel.load_entries()?;

    // Select the file
    for (i, entry) in app.left_panel.entries.iter().enumerate() {
        if entry.name == "test.txt" {
            app.left_panel.select(Some(i));
            break;
        }
    }

    handle_view(&mut app)?;

    assert!(app.get_viewer().is_some());
    let viewer = app.get_viewer().unwrap();
    assert_eq!(viewer.title(), "test.txt");
    assert_eq!(viewer.lines.len(), 3);
    assert_eq!(viewer.lines[0], "Test content");
    assert_eq!(viewer.scroll, ScrollTop::TOP);

    Ok(())
}

#[test]
fn test_handle_copy_file() -> io::Result<()> {
    let temp_dir = TempDir::new()?;
    let mut app = App::new()?;

    let left_dir = temp_dir.path().join("left");
    let right_dir = temp_dir.path().join("right");
    fs::create_dir(&left_dir)?;
    fs::create_dir(&right_dir)?;

    // Create test file
    File::create(left_dir.join("test.txt"))?.write_all(b"content")?;

    app.left_panel.path = left_dir;
    app.right_panel.path = right_dir.clone();
    app.left_panel.load_entries()?;
    app.right_panel.load_entries()?;

    // Select the file
    for (i, entry) in app.left_panel.entries.iter().enumerate() {
        if entry.name == "test.txt" {
            app.left_panel.select(Some(i));
            break;
        }
    }

    handle_copy(&mut app)?;

    // Verify file was copied
    assert!(right_dir.join("test.txt").exists());
    let content = fs::read_to_string(right_dir.join("test.txt"))?;
    assert_eq!(content, "content");

    Ok(())
}

#[test]
fn test_handle_move_file() -> io::Result<()> {
    let temp_dir = TempDir::new()?;
    let mut app = App::new()?;

    let left_dir = temp_dir.path().join("left");
    let right_dir = temp_dir.path().join("right");
    fs::create_dir(&left_dir)?;
    fs::create_dir(&right_dir)?;

    // Create test file
    let src_file = left_dir.join("test.txt");
    File::create(&src_file)?.write_all(b"content")?;

    app.left_panel.path = left_dir;
    app.right_panel.path = right_dir.clone();
    app.left_panel.load_entries()?;
    app.right_panel.load_entries()?;

    // Select the file
    for (i, entry) in app.left_panel.entries.iter().enumerate() {
        if entry.name == "test.txt" {
            app.left_panel.select(Some(i));
            break;
        }
    }

    handle_move(&mut app)?;

    // Verify file was moved
    assert!(!src_file.exists());
    assert!(right_dir.join("test.txt").exists());
    let content = fs::read_to_string(right_dir.join("test.txt"))?;
    assert_eq!(content, "content");

    Ok(())
}

#[test]
fn test_handle_copy_single_file_only() -> io::Result<()> {
    // Note: handle_copy only copies the selected file, not marked files
    let temp_dir = TempDir::new()?;
    let mut app = App::new()?;

    let left_dir = temp_dir.path().join("left");
    let right_dir = temp_dir.path().join("right");
    fs::create_dir(&left_dir)?;
    fs::create_dir(&right_dir)?;

    // Create test files
    File::create(left_dir.join("file1.txt"))?.write_all(b"content1")?;
    File::create(left_dir.join("file2.txt"))?.write_all(b"content2")?;

    app.left_panel.path = left_dir;
    app.right_panel.path = right_dir.clone();
    app.left_panel.load_entries()?;
    app.right_panel.load_entries()?;

    // Select file1.txt
    for (i, entry) in app.left_panel.entries.iter().enumerate() {
        if entry.name == "file1.txt" {
            app.left_panel.select(Some(i));
            break;
        }
    }

    // If we mark file2.txt, it WILL be copied now because marked items are prioritized
    app.left_panel.marked.insert(FileName::from("file2.txt"));

    handle_copy(&mut app)?;

    // Verify marked file was copied, selected one (file1.txt) was NOT (because prioritization)
    assert!(!right_dir.join("file1.txt").exists());
    assert!(right_dir.join("file2.txt").exists());

    Ok(())
}
