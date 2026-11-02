//! Purpose: Recursive directory copy and panel refresh.

use super::*;

#[test]
fn test_copy_dir_all_actually_copies() -> io::Result<()> {
    // Kill: replace copy_dir_all -> io::Result<()> with Ok(())
    let temp_dir = TempDir::new()?;
    let src = temp_dir.path().join("src");
    let dst = temp_dir.path().join("dst");

    // Create source structure
    fs::create_dir(&src)?;
    File::create(src.join("file1.txt"))?.write_all(b"content1")?;
    fs::create_dir(src.join("subdir"))?;
    File::create(src.join("subdir").join("file2.txt"))?.write_all(b"content2")?;

    // Perform copy
    copy_dir_all(&src, &dst)?;

    // Verify files were actually copied
    assert!(dst.exists(), "Destination directory should exist");
    assert!(dst.is_dir(), "Destination should be a directory");

    let file1_content = fs::read_to_string(dst.join("file1.txt"))?;
    assert_eq!(file1_content, "content1", "File content should match");

    assert!(dst.join("subdir").exists(), "Subdirectory should be copied");
    let file2_content = fs::read_to_string(dst.join("subdir").join("file2.txt"))?;
    assert_eq!(
        file2_content, "content2",
        "Nested file content should match"
    );

    Ok(())
}

#[test]
fn test_copy_dir_all_empty_dir() -> io::Result<()> {
    // Additional test to ensure empty directories are handled
    let temp_dir = TempDir::new()?;
    let src = temp_dir.path().join("empty_src");
    let dst = temp_dir.path().join("empty_dst");

    fs::create_dir(&src)?;
    copy_dir_all(&src, &dst)?;

    assert!(dst.exists());
    assert!(dst.is_dir());
    assert_eq!(fs::read_dir(&dst)?.count(), 0);

    Ok(())
}

#[test]
fn test_refresh_panels_actually_refreshes() -> io::Result<()> {
    // Kill: replace refresh_panels -> io::Result<()> with Ok(())
    let temp_dir = TempDir::new()?;
    let mut app = App::new()?;

    // Set both panels to the temp directory
    app.left_panel.path = temp_dir.path().to_path_buf();
    app.right_panel.path = temp_dir.path().to_path_buf();

    // Initial load
    app.left_panel.load_entries()?;
    app.right_panel.load_entries()?;
    let initial_left_count = app.left_panel.entries.len();
    let initial_right_count = app.right_panel.entries.len();

    // Add multiple new files
    File::create(temp_dir.path().join("new_file_1.txt"))?;
    File::create(temp_dir.path().join("new_file_2.txt"))?;
    fs::create_dir(temp_dir.path().join("new_dir"))?;

    // Refresh panels
    refresh_panels(&mut app)?;

    // Verify panels were refreshed with new content
    assert!(
        app.left_panel.entries.len() > initial_left_count,
        "Left panel should have more entries after refresh"
    );
    assert!(
        app.right_panel.entries.len() > initial_right_count,
        "Right panel should have more entries after refresh"
    );

    // Verify specific new entries exist
    for panel in [&app.left_panel, &app.right_panel] {
        for name in ["new_file_1.txt", "new_file_2.txt", "new_dir"] {
            assert!(panel.entries.iter().any(|entry| entry.name == name));
        }
    }

    // Verify selections are maintained (ensure_selection is called)
    assert!(app.left_panel.selected().is_some());
    assert!(app.right_panel.selected().is_some());

    Ok(())
}

#[test]
fn test_refresh_panels_handles_deleted_files() -> io::Result<()> {
    // Additional test for refresh behavior with deleted files
    let temp_dir = TempDir::new()?;
    let mut app = App::new()?;

    app.left_panel.path = temp_dir.path().to_path_buf();
    app.right_panel.path = temp_dir.path().to_path_buf();

    // Create initial files
    File::create(temp_dir.path().join("temp1.txt"))?;
    File::create(temp_dir.path().join("temp2.txt"))?;

    // Initial load
    refresh_panels(&mut app)?;
    let initial_count = app.left_panel.entries.len();

    // Delete a file
    fs::remove_file(temp_dir.path().join("temp1.txt"))?;

    // Refresh again
    refresh_panels(&mut app)?;

    // Verify the file list was updated
    assert!(app.left_panel.entries.len() < initial_count);
    let names: Vec<_> = app.left_panel.entries.iter().map(|e| &e.name).collect();
    assert!(!names.contains(&&"temp1.txt".to_owned()));
    assert!(names.contains(&&"temp2.txt".to_owned()));

    Ok(())
}
