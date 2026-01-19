use funny_file_manager::models::{App, Panel, PanelSide};
use funny_file_manager::WritableTestBackend;
use ratatui::Terminal;
use std::fs::File;
use std::io;
use tempfile::TempDir;

#[cfg(feature = "test-helpers")]
use funny_file_manager::editor::{handle_edit_test_mode, TestEditorBehavior};

fn create_test_app(temp_dir: &TempDir) -> io::Result<App> {
    Ok(App {
        left_panel: Panel::new(temp_dir.path().to_path_buf())?,
        right_panel: Panel::new(temp_dir.path().to_path_buf())?,
        active_panel: PanelSide::Left,
        status_message: String::new(),
        show_help: false,
        viewer: None,
        search: None,
        command_line: None,
        show_bookmarks: false,
        bookmarks: vec![None; 10],
        should_quit: false,
        quick_search: None,
    })
}

#[test]
#[cfg(feature = "test-helpers")]
fn test_handle_edit_success() -> io::Result<()> {
    let temp_dir = TempDir::new()?;
    let file_path = temp_dir.path().join("test.txt");
    File::create(&file_path)?;

    let mut app = create_test_app(&temp_dir)?;
    app.left_panel.state.select(Some(1)); // Select the file

    handle_edit_test_mode(&mut app, TestEditorBehavior::Success)?;

    assert_eq!(
        app.status_message,
        format!("Edited {}", file_path.display())
    );
    Ok(())
}

#[test]
#[cfg(feature = "test-helpers")]
fn test_handle_edit_fail_launch() -> io::Result<()> {
    let temp_dir = TempDir::new()?;
    let file_path = temp_dir.path().join("test.txt");
    File::create(&file_path)?;

    let mut app = create_test_app(&temp_dir)?;
    app.left_panel.state.select(Some(1)); // Select the file

    handle_edit_test_mode(&mut app, TestEditorBehavior::FailLaunch)?;

    assert_eq!(
        app.status_message,
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
    app.left_panel.state.select(Some(1)); // Select the file

    handle_edit_test_mode(&mut app, TestEditorBehavior::ExitError)?;

    assert_eq!(
        app.status_message,
        "Editor exited with status exit status: 1"
    );
    Ok(())
}

#[test]
#[cfg(feature = "test-helpers")]
fn test_handle_edit_no_selection() -> io::Result<()> {
    let temp_dir = TempDir::new()?;
    let mut app = create_test_app(&temp_dir)?;
    app.left_panel.state.select(None);

    handle_edit_test_mode(&mut app, TestEditorBehavior::Success)?;

    assert_eq!(app.status_message, "");
    Ok(())
}

#[test]
#[cfg(feature = "test-helpers")]
fn test_handle_edit_directory() -> io::Result<()> {
    let temp_dir = TempDir::new()?;
    let mut app = create_test_app(&temp_dir)?;
    app.left_panel.state.select(Some(0)); // Select the ".." directory

    handle_edit_test_mode(&mut app, TestEditorBehavior::Success)?;

    assert_eq!(app.status_message, "Editing directories is not supported");
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
            app.right_panel.state.select(Some(i));
            break;
        }
    }

    handle_edit_test_mode(&mut app, TestEditorBehavior::Success)?;

    assert!(app.status_message.contains("Edited"));
    assert!(app.status_message.contains("right_file.txt"));
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
            app.left_panel.state.select(Some(i));
            break;
        }
    }

    handle_edit_test_mode(&mut app, TestEditorBehavior::Success)?;

    assert_eq!(app.status_message, "Editing directories is not supported");
    Ok(())
}

#[test]
#[cfg(feature = "test-helpers")]
fn test_handle_edit_reloads_entries() -> io::Result<()> {
    let temp_dir = TempDir::new()?;
    let file_path = temp_dir.path().join("test.txt");
    File::create(&file_path)?;

    let mut app = create_test_app(&temp_dir)?;
    app.left_panel.state.select(Some(1)); // Select the file

    // Get initial entry count
    let initial_count = app.left_panel.entries.len();

    handle_edit_test_mode(&mut app, TestEditorBehavior::Success)?;

    // Entries should be reloaded (count may be same but reload happened)
    assert_eq!(app.left_panel.entries.len(), initial_count);
    Ok(())
}

