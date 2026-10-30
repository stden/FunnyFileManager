//! Purpose: Command-line cd, bookmark and mark commands given bad or empty input.

use super::*;

#[test]
fn test_cmd_cd_invalid_path() -> io::Result<()> {
    let temp_dir = TempDir::new()?;
    let mut app = App::new()?;
    app.left_panel = Panel::new(temp_dir.path().to_path_buf())?;

    // Non-existent path should fail canonicalize
    app.set_overlay(Overlay::CommandLine("cd non_existent_path_xyz".into()));
    process_popup_input(&mut app, key_event(KeyCode::Enter));

    assert!(app.status_text().contains("Error"));
    Ok(())
}

#[test]
fn test_cmd_cd_to_file() -> io::Result<()> {
    let temp_dir = TempDir::new()?;
    let file_path = temp_dir.path().join("test.txt");
    File::create(&file_path)?;

    let mut app = App::new()?;
    app.left_panel = Panel::new(temp_dir.path().to_path_buf())?;

    app.set_overlay(Overlay::CommandLine(
        format!("cd {}", file_path.display()).into(),
    ));
    process_popup_input(&mut app, key_event(KeyCode::Enter));

    assert!(app.status_text().contains("Error"));
    assert!(app.status_text().contains("not a directory"));
    Ok(())
}

#[test]
fn test_cmd_bookmark_invalid_slot() -> io::Result<()> {
    let mut app = App::new()?;
    app.set_overlay(Overlay::CommandLine("bookmark set 10".into()));
    process_popup_input(&mut app, key_event(KeyCode::Enter));
    assert!(app.status_text().contains("Error"));

    app.set_overlay(Overlay::CommandLine("bookmark set abc".into()));
    process_popup_input(&mut app, key_event(KeyCode::Enter));
    assert!(app.status_text().contains("Error"));
    Ok(())
}

#[test]
fn test_cmd_bookmark_go_empty() -> io::Result<()> {
    let mut app = App::new()?;
    app.set_overlay(Overlay::CommandLine("bookmark go 5".into()));
    process_popup_input(&mut app, key_event(KeyCode::Enter));
    assert!(app.status_text().contains("Error"));
    assert!(app.status_text().contains("empty"));
    Ok(())
}

#[test]
fn test_cmd_mark_invalid() -> io::Result<()> {
    let mut app = App::new()?;
    app.set_overlay(Overlay::CommandLine("mark unknown".into()));
    process_popup_input(&mut app, key_event(KeyCode::Enter));
    assert!(app.status_text().contains("Error"));
    Ok(())
}

#[test]
fn test_cmd_mark_all_empty() -> io::Result<()> {
    let temp_dir = TempDir::new()?;
    // Directory with only ".."
    let mut app = App::new()?;
    app.left_panel = Panel::new(temp_dir.path().to_path_buf())?;
    // Clear any files if App::new used current dir
    app.left_panel.entries.retain(|e| e.name == "..");

    app.set_overlay(Overlay::CommandLine("mark all".into()));
    process_popup_input(&mut app, key_event(KeyCode::Enter));

    assert_eq!(app.left_panel.marked.len(), 0);
    Ok(())
}
