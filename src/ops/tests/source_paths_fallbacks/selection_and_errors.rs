//! Purpose: Source paths without marks, and a failed command's status.

use super::*;

#[test]
fn test_get_source_paths_no_marks_uses_selected() -> io::Result<()> {
    let temp_dir = TempDir::new()?;
    fs::write(temp_dir.path().join("only.txt"), "data")?;
    let mut app = App::new()?;
    app.left_panel = Panel::new(temp_dir.path().to_path_buf())?;

    // Select the file
    for (i, entry) in app.left_panel.entries.iter().enumerate() {
        if entry.name == "only.txt" {
            app.left_panel.select(Some(i));
            break;
        }
    }

    let paths = get_source_paths(&app);
    assert_eq!(paths.len(), 1);
    assert!(paths[0].ends_with("only.txt"));

    Ok(())
}

#[test]
fn test_get_source_paths_no_marks_no_selection() -> io::Result<()> {
    let temp_dir = TempDir::new()?;
    let mut app = App::new()?;
    app.left_panel = Panel::new(temp_dir.path().to_path_buf())?;
    app.left_panel.entries.clear();
    app.left_panel.select(None);

    let paths = get_source_paths(&app);
    assert_eq!(paths.len(), 0);

    Ok(())
}

// -----------------------------------------------------------------------
// execute_command: error propagation through command line
// -----------------------------------------------------------------------

#[test]
fn test_execute_command_error_sets_status() -> io::Result<()> {
    let temp_dir = TempDir::new()?;
    let mut app = App::new()?;
    app.left_panel = Panel::new(temp_dir.path().to_path_buf())?;
    app.right_panel = Panel::new(temp_dir.path().to_path_buf())?;

    // Enter command line mode, type an invalid command, submit
    app.set_overlay(Overlay::CommandLine("cd /nonexistent_path_99999".into()));
    let key = key_event(KeyCode::Enter);
    process_popup_input(&mut app, key);
    assert!(
        app.status_text().contains("Error"),
        "status should contain error: {}",
        app.status_text()
    );
    Ok(())
}
