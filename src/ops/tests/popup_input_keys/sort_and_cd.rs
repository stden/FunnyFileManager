//! Purpose: exit, sort's defaults and long forms, and `cd` onto a file.

use super::*;

// -----------------------------------------------------------------------
// execute_command -- additional branch coverage
// -----------------------------------------------------------------------

/// `exit` is an alias for `quit` and sets `should_quit`.
#[test]
fn test_execute_command_exit_alias() -> io::Result<()> {
    let mut app = App::new().expect("test setup");

    execute_command(&mut app, "exit").expect("exit command");

    assert!(!app.lifecycle.is_running());

    Ok(())
}

/// `sort` with no argument defaults to sorting by name.
#[test]
fn test_execute_command_sort_no_arg_defaults_to_name() -> io::Result<()> {
    let temp_dir = TempDir::new().expect("test setup");
    let mut app = App::new().expect("test setup");
    app.left_panel = Panel::new(temp_dir.path().to_path_buf()).expect("test setup");

    // Set to a different mode first so the change back to Name is observable.
    app.get_active_panel().sort_mode = SortMode::Size;

    execute_command(&mut app, "sort").expect("sort without arg");

    assert_eq!(app.get_active_panel().sort_mode, SortMode::Name);

    Ok(())
}

/// `sort extension` (long form) sets mode to Extension.
#[test]
fn test_execute_command_sort_extension_long_form() -> io::Result<()> {
    let temp_dir = TempDir::new().expect("test setup");
    let mut app = App::new().expect("test setup");
    app.left_panel = Panel::new(temp_dir.path().to_path_buf()).expect("test setup");

    execute_command(&mut app, "sort extension").expect("sort extension");

    assert_eq!(app.get_active_panel().sort_mode, SortMode::Extension);

    Ok(())
}

#[test]
fn test_execute_command_cd_to_file_not_dir() -> io::Result<()> {
    let temp_dir = TempDir::new().expect("test setup");
    let file_path = temp_dir.path().join("notadir.txt");
    fs::write(&file_path, b"data").expect("test setup");

    let mut app = App::new().expect("test setup");
    app.left_panel = Panel::new(temp_dir.path().to_path_buf()).expect("test setup");

    let result = execute_command(&mut app, &format!("cd {}", file_path.display()));
    assert!(result.is_err());

    Ok(())
}

#[test]
fn test_execute_command_sort_extension() -> io::Result<()> {
    let temp_dir = TempDir::new().expect("test setup");
    fs::write(temp_dir.path().join("b.txt"), b"b").expect("test setup");
    fs::write(temp_dir.path().join("a.rs"), b"a").expect("test setup");

    let mut app = App::new().expect("test setup");
    app.left_panel = Panel::new(temp_dir.path().to_path_buf()).expect("test setup");

    execute_command(&mut app, "sort ext").expect("test setup");
    assert_eq!(app.left_panel.sort_mode, SortMode::Extension);
    assert!(app.status_text().contains("Ext"));

    execute_command(&mut app, "sort extension").expect("test setup");
    assert_eq!(app.left_panel.sort_mode, SortMode::Extension);

    execute_command(&mut app, "sort size").expect("test setup");
    assert_eq!(app.left_panel.sort_mode, SortMode::Size);
    assert!(app.status_text().contains("Size"));

    execute_command(&mut app, "sort name").expect("test setup");
    assert_eq!(app.left_panel.sort_mode, SortMode::Name);
    assert!(app.status_text().contains("Name"));

    Ok(())
}
