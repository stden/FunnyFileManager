//! Purpose: Feature-gated tests of the paths `handle_edit_impl` hands the editor: the
//! right panel, a missing file and a very long name.
use super::editor_tests::create_test_app;
use crate::editor::handle_edit_impl;
use crate::models::EntryKind;
use crate::models::FileSize;
use crate::models::Modified;
use crate::models::{App, PanelSide};
use ratatui::Terminal;
use std::fs::File;
use std::io;
use tempfile::TempDir;

#[test]
fn test_handle_edit_impl_right_panel() -> io::Result<()> {
    let temp_dir = TempDir::new()?;
    let file_path = temp_dir.path().join("right.txt");
    File::create(&file_path)?;

    let mut app = create_test_app(&temp_dir)?;
    app.active_panel = PanelSide::Right;

    // Find and select the file in right panel
    for (i, entry) in app.right_panel.entries.iter().enumerate() {
        if entry.name == "right.txt" {
            app.right_panel.select(Some(i));
            break;
        }
    }

    let backend = crate::WritableTestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend)?;

    let result = handle_edit_impl(&mut terminal, &mut app, |_, _, path| {
        assert_eq!(path.file_name().unwrap().to_str().unwrap(), "right.txt");
        Ok(std::process::ExitStatus::default())
    });

    assert!(result.is_ok());
    assert!(app.status_text().contains("Edited"));
    Ok(())
}

#[test]
fn test_handle_edit_impl_file_not_found() -> io::Result<()> {
    let temp_dir = TempDir::new()?;
    let mut app = App::new()?;

    app.left_panel.path = temp_dir.path().to_path_buf();
    app.left_panel.load_entries()?;

    // Select a non-existent file by manipulating the panel state
    app.left_panel.entries.push(crate::models::DirEntry {
        name: "nonexistent.txt".to_string(),
        kind: EntryKind::File,
        size: FileSize::from_bytes(0),
        modified: Modified::UNKNOWN,
        executable: false,
        ..crate::models::DirEntry::default()
    });
    app.left_panel
        .select(Some(app.left_panel.entries.len() - 1));

    let backend = crate::WritableTestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend)?;

    let result = handle_edit_impl(&mut terminal, &mut app, |_editor, _args, path| {
        // Verify the path doesn't exist
        assert!(!path.exists());
        Ok(std::process::ExitStatus::default())
    });

    assert!(result.is_ok());
    Ok(())
}

#[test]
fn test_handle_edit_impl_very_long_filename() -> io::Result<()> {
    let temp_dir = TempDir::new()?;
    let mut app = App::new()?;

    // Create a file with a long name (but valid for Windows)
    let long_name = "a".repeat(100) + ".txt";
    let file = temp_dir.path().join(&long_name);
    File::create(&file)?;

    app.left_panel.path = temp_dir.path().to_path_buf();
    app.left_panel.load_entries()?;

    // Find and select the long filename
    for (i, entry) in app.left_panel.entries.iter().enumerate() {
        if entry.name.contains("aaaa") {
            app.left_panel.select(Some(i));
            break;
        }
    }

    let backend = crate::WritableTestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend)?;

    let result = handle_edit_impl(&mut terminal, &mut app, |_editor, _args, path| {
        assert!(path.file_name().unwrap().len() > 100);
        Ok(std::process::ExitStatus::default())
    });

    assert!(result.is_ok());
    Ok(())
}
