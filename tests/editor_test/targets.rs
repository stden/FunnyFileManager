//! Purpose: What F4 edits: no selection, a directory, the right panel and a subdirectory.

use super::*;

#[test]
#[cfg(feature = "test-helpers")]
fn test_handle_edit_no_selection() -> io::Result<()> {
    let temp_dir = TempDir::new()?;
    let mut app = create_test_app(&temp_dir)?;
    app.left_panel.select(None);

    handle_edit_test_mode(&mut app, TestEditorBehavior::Success)?;

    assert_eq!(app.status_text(), "");
    Ok(())
}

#[test]
#[cfg(feature = "test-helpers")]
fn test_handle_edit_directory() -> io::Result<()> {
    let temp_dir = TempDir::new()?;
    let mut app = create_test_app(&temp_dir)?;
    app.left_panel.select(Some(0)); // Select the ".." directory

    handle_edit_test_mode(&mut app, TestEditorBehavior::Success)?;

    assert_eq!(app.status_text(), "Editing directories is not supported");
    Ok(())
}

#[test]
#[cfg(feature = "test-helpers")]
fn test_handle_edit_selected_path_none() -> io::Result<()> {
    let temp_dir = TempDir::new()?;
    let mut app = create_test_app(&temp_dir)?;
    *app.left_panel.entries = vec![DirEntry {
        name: "..".to_owned(),
        kind: EntryKind::File,
        size: FileSize::from_bytes(0),
        modified: Modified::UNKNOWN,
        executable: false,
        ..DirEntry::default()
    }];
    app.left_panel.select(Some(0));

    handle_edit_test_mode(&mut app, TestEditorBehavior::Success)?;

    assert_eq!(app.status_text(), "");
    Ok(())
}

#[test]
#[cfg(feature = "test-helpers")]
fn test_handle_edit_right_panel() -> io::Result<()> {
    let temp_dir = TempDir::new()?;
    let file_path = temp_dir.path().join("right_file.txt");
    File::create(&file_path)?;

    let mut app = create_test_app(&temp_dir)?;
    app.active_panel = PanelSide::Right;
    app.right_panel = Panel::new(temp_dir.path().to_path_buf())?;

    // Select the file in right panel
    for (i, entry) in app.right_panel.entries.iter().enumerate() {
        if entry.name == "right_file.txt" {
            app.right_panel.select(Some(i));
            break;
        }
    }

    handle_edit_test_mode(&mut app, TestEditorBehavior::Success)?;

    assert!(app.status_text().contains("Edited"));
    assert!(app.status_text().contains("right_file.txt"));
    Ok(())
}

#[test]
#[cfg(feature = "test-helpers")]
fn test_handle_edit_subdirectory() -> io::Result<()> {
    let temp_dir = TempDir::new()?;
    let subdir = temp_dir.path().join("subdir");
    std::fs::create_dir(&subdir)?;

    let mut app = create_test_app(&temp_dir)?;

    // Select the subdir
    for (i, entry) in app.left_panel.entries.iter().enumerate() {
        if entry.name == "subdir" {
            app.left_panel.select(Some(i));
            break;
        }
    }

    handle_edit_test_mode(&mut app, TestEditorBehavior::Success)?;

    assert_eq!(app.status_text(), "Editing directories is not supported");
    Ok(())
}
