//! Purpose: Typed commands with a missing or invalid argument, and unknown ones.

use super::*;

#[test]
fn test_execute_command_edge_cases() -> io::Result<()> {
    let temp_dir = TempDir::new().expect("test setup");
    let mut app = App::new().expect("test setup");
    app.left_panel.path = temp_dir.path().to_path_buf();

    assert!(execute_command(&mut app, "").is_err());
    assert!(execute_command(&mut app, "   ").is_err());
    // "unknown_command" is dispatched to the shell; the process runs (Ok)
    // even though the command itself is not found by the shell.
    assert!(execute_command(&mut app, "unknown_command").is_ok());

    execute_command(&mut app, "  help  ").expect("test setup");
    assert!(matches!(app.overlay, Overlay::Help));
    app.close_overlay();

    // "HELP" (uppercase) is not a recognized built-in, so it runs as a
    // shell command and returns Ok.
    assert!(execute_command(&mut app, "HELP").is_ok());

    Ok(())
}

#[test]
fn test_execute_command_cd_missing_argument() -> io::Result<()> {
    let mut app = App::new().expect("test setup");

    let result = execute_command(&mut app, "cd");
    assert!(result.is_err());
    assert_eq!(result.unwrap_err().kind(), io::ErrorKind::InvalidInput);

    Ok(())
}

#[test]
fn test_execute_command_cd_invalid_path() -> io::Result<()> {
    let temp_dir = TempDir::new().expect("test setup");
    let mut app = App::new().expect("test setup");
    app.left_panel.path = temp_dir.path().to_path_buf();

    // Try to cd to a file instead of directory
    let file = temp_dir.path().join("file.txt");
    File::create(&file).expect("test setup");

    let result = execute_command(&mut app, &format!("cd {}", file.display()));
    assert!(result.is_err());

    Ok(())
}

#[test]
fn test_execute_command_sort_invalid_mode() -> io::Result<()> {
    let mut app = App::new().expect("test setup");

    let result = execute_command(&mut app, "sort invalid");
    assert!(result.is_err());
    assert_eq!(result.unwrap_err().kind(), io::ErrorKind::InvalidInput);

    Ok(())
}

#[test]
fn test_execute_command_bookmark_missing_subcommand() -> io::Result<()> {
    let mut app = App::new().expect("test setup");

    let result = execute_command(&mut app, "bookmark");
    assert!(result.is_err());
    assert_eq!(result.unwrap_err().kind(), io::ErrorKind::InvalidInput);

    Ok(())
}

#[test]
fn test_execute_command_bookmark_invalid_slot() -> io::Result<()> {
    let mut app = App::new().expect("test setup");

    // With BookmarkSlot + Command::Invalid, bad slots produce errors (exact message may vary).
    assert!(execute_command(&mut app, "bookmark set 10").is_err());
    assert!(execute_command(&mut app, "bookmark set abc").is_err());

    Ok(())
}

#[test]
fn test_execute_command_bookmark_go_empty() -> io::Result<()> {
    let mut app = App::new().expect("test setup");

    // Try to go to an empty bookmark
    let result = execute_command(&mut app, "bookmark go 0");
    assert!(result.is_err());
    let err_msg = result.unwrap_err().to_string();
    assert!(err_msg.contains("Bookmark") || err_msg.contains("empty"));

    Ok(())
}

#[test]
fn test_execute_command_bookmark_invalid_subcommand() -> io::Result<()> {
    let mut app = App::new().expect("test setup");

    let result = execute_command(&mut app, "bookmark invalid 0");
    assert!(result.is_err());
    // Just check that it returns an error
    assert!(result.is_err());

    Ok(())
}

#[test]
fn test_execute_command_mark_missing_argument() -> io::Result<()> {
    let mut app = App::new().expect("test setup");

    let result = execute_command(&mut app, "mark");
    assert!(result.is_err());
    assert_eq!(result.unwrap_err().kind(), io::ErrorKind::InvalidInput);

    Ok(())
}

#[test]
fn test_execute_command_mark_invalid_subcommand() -> io::Result<()> {
    let temp_dir = TempDir::new().expect("test setup");
    let mut app = App::new().expect("test setup");
    app.left_panel.path = temp_dir.path().to_path_buf();
    app.left_panel.load_entries().expect("test setup");

    let result = execute_command(&mut app, "mark invalid");
    assert!(result.is_err());
    // Just check that it returns an error
    assert!(result.is_err());

    Ok(())
}

#[test]
fn test_execute_command_unknown_command() -> io::Result<()> {
    let mut app = App::new().expect("test setup");

    // Unknown commands are dispatched to the shell, which returns Ok
    // (the shell process itself runs successfully even if the command
    // is not found).
    let result = execute_command(&mut app, "unknowncommand arg1 arg2");
    assert!(result.is_ok());
    // The status_message should reflect the shell error output
    assert_ne!(app.status_text().len(), 0);

    Ok(())
}
