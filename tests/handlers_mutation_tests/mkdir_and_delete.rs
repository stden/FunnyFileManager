//! Purpose: The make-folder and delete handlers on files and directories.

use super::*;

#[test]
fn test_handle_mkdir() -> io::Result<()> {
    let temp_dir = TempDir::new()?;
    let mut app = App::new()?;

    app.left_panel.path = temp_dir.path().to_path_buf();
    app.left_panel.load_entries()?;

    let initial_count = app.left_panel.entries.len();

    handle_mkdir(&mut app)?;

    // Verify directory was created (will be named "new_folder" or "new_folder_N")
    let new_dir = temp_dir.path().join("new_folder");
    assert!(new_dir.exists() || temp_dir.path().join("new_folder_1").exists());
    assert!(new_dir.is_dir() || temp_dir.path().join("new_folder_1").is_dir());

    // Verify panel was refreshed
    assert!(app.left_panel.entries.len() > initial_count);
    let names: Vec<_> = app.left_panel.entries.iter().map(|e| &e.name).collect();
    assert!(
        names.contains(&&"new_folder".to_owned()) || names.contains(&&"new_folder_1".to_owned())
    );

    Ok(())
}

#[test]
fn test_handle_delete_file() -> io::Result<()> {
    let temp_dir = TempDir::new()?;
    let mut app = App::new()?;

    // Create test file
    let file_path = temp_dir.path().join("to_delete.txt");
    File::create(&file_path)?.write_all(b"delete me")?;

    app.left_panel.path = temp_dir.path().to_path_buf();
    app.left_panel.load_entries()?;

    // Select the file
    for (i, entry) in app.left_panel.entries.iter().enumerate() {
        if entry.name == "to_delete.txt" {
            app.left_panel.select(Some(i));
            break;
        }
    }

    handle_delete_confirm(&mut app)?;
    handle_delete(&mut app)?;

    // Verify file was deleted
    assert!(!file_path.exists());

    // Verify panel was refreshed

    assert!(!app
        .left_panel
        .entries
        .iter()
        .map(|e| &e.name)
        .any(|x| x == &"to_delete.txt".to_owned()));

    Ok(())
}

#[test]
fn test_handle_delete_directory() -> io::Result<()> {
    let temp_dir = TempDir::new()?;
    let mut app = App::new()?;

    // Create test directory with content
    let dir_path = temp_dir.path().join("to_delete");
    fs::create_dir(&dir_path)?;
    File::create(dir_path.join("file.txt"))?.write_all(b"content")?;

    app.left_panel.path = temp_dir.path().to_path_buf();
    app.left_panel.load_entries()?;

    // Select the directory
    for (i, entry) in app.left_panel.entries.iter().enumerate() {
        if entry.name == "to_delete" {
            app.left_panel.select(Some(i));
            break;
        }
    }

    handle_delete_confirm(&mut app)?;
    handle_delete(&mut app)?;

    // Verify directory was deleted
    assert!(!dir_path.exists());

    // Verify panel was refreshed

    assert!(!app
        .left_panel
        .entries
        .iter()
        .map(|e| &e.name)
        .any(|x| x == &"to_delete".to_owned()));

    Ok(())
}
