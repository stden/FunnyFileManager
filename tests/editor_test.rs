//! Purpose: Exercises editor workflows from the public integration-test boundary.
//!
use funny_file_manager::models::EntryKind;
use funny_file_manager::models::FileSize;
use funny_file_manager::models::Modified;
use funny_file_manager::models::{App, DirEntry, Panel, PanelSide};
use std::fs::File;
use std::io;
use tempfile::TempDir;

#[cfg(feature = "test-helpers")]
use funny_file_manager::editor::{handle_edit_test_mode, TestEditorBehavior};

fn create_test_app(temp_dir: &TempDir) -> io::Result<App> {
    let mut app = App::with_panels(
        Panel::new(temp_dir.path().to_path_buf())?,
        Panel::new(temp_dir.path().to_path_buf())?,
    );
    app.status = funny_file_manager::status::Status::Cleared;
    Ok(app)
}

#[test]
#[cfg(feature = "test-helpers")]
fn test_handle_edit_success() -> io::Result<()> {
    let temp_dir = TempDir::new()?;
    let file_path = temp_dir.path().join("test.txt");
    File::create(&file_path)?;

    let mut app = create_test_app(&temp_dir)?;
    app.left_panel.select(Some(1)); // Select the file

    handle_edit_test_mode(&mut app, TestEditorBehavior::Success)?;

    assert_eq!(app.status_text(), format!("Edited {}", file_path.display()));
    Ok(())
}

#[test]
#[cfg(feature = "test-helpers")]
fn test_handle_edit_fail_launch() -> io::Result<()> {
    let temp_dir = TempDir::new()?;
    let file_path = temp_dir.path().join("test.txt");
    File::create(&file_path)?;

    let mut app = create_test_app(&temp_dir)?;
    app.left_panel.select(Some(1)); // Select the file

    handle_edit_test_mode(&mut app, TestEditorBehavior::FailLaunch)?;

    assert_eq!(
        app.status_text(),
        "Failed to launch editor: No such file or directory (os error 2)"
    );
    Ok(())
}

#[test]
#[cfg(feature = "test-helpers")]
fn test_handle_edit_exit_error() -> io::Result<()> {
    let temp_dir = TempDir::new()?;
    let file_path = temp_dir.path().join("test.txt");
    File::create(&file_path)?;

    let mut app = create_test_app(&temp_dir)?;
    app.left_panel.select(Some(1)); // Select the file

    handle_edit_test_mode(&mut app, TestEditorBehavior::ExitError)?;

    assert_eq!(
        app.status_text(),
        "Editor exited with status exit status: 1"
    );
    Ok(())
}

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
    app.left_panel.entries = vec![DirEntry {
        name: "..".to_string(),
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

#[test]
#[cfg(feature = "test-helpers")]
fn test_handle_edit_reloads_entries() -> io::Result<()> {
    let temp_dir = TempDir::new()?;
    let file_path = temp_dir.path().join("test.txt");
    File::create(&file_path)?;

    let mut app = create_test_app(&temp_dir)?;
    app.left_panel.select(Some(1)); // Select the file

    // Get initial entry count
    let initial_count = app.left_panel.entries.len();

    handle_edit_test_mode(&mut app, TestEditorBehavior::Success)?;

    // Entries should be reloaded (count may be same but reload happened)
    assert_eq!(app.left_panel.entries.len(), initial_count);
    Ok(())
}

#[test]
#[cfg(feature = "test-helpers")]
fn test_handle_edit_propagates_reload_error() -> io::Result<()> {
    let temp_dir = TempDir::new()?;
    let file_path = temp_dir.path().join("right_file.txt");
    File::create(&file_path)?;

    let mut app = create_test_app(&temp_dir)?;
    app.active_panel = PanelSide::Right;
    app.right_panel = Panel::new(temp_dir.path().to_path_buf())?;
    app.left_panel.path = temp_dir.path().join("missing-left-panel");

    for (i, entry) in app.right_panel.entries.iter().enumerate() {
        if entry.name == "right_file.txt" {
            app.right_panel.select(Some(i));
            break;
        }
    }

    let err = handle_edit_test_mode(&mut app, TestEditorBehavior::Success)
        .expect_err("missing left panel path should fail reload");

    assert_eq!(err.kind(), io::ErrorKind::NotFound);
    assert!(app.status_text().contains("Edited"));
    Ok(())
}

// The library dependency retains the real probe, which cfg(test) unit tests skip.
#[cfg(all(unix, not(coverage), feature = "test-helpers"))]
#[path = "support/editor_probe.rs"]
mod editor_probe;

#[cfg(all(unix, not(coverage), feature = "test-helpers"))]
#[test]
fn handle_edit_probes_for_the_code_cli_and_uses_it_when_present() -> io::Result<()> {
    editor_probe::verify()
}
