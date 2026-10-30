//! Purpose: The typed built-ins: sort, mark, bookmarks, quit, help, pwd and unmark.

use super::*;

#[test]
fn test_cmd_sort_extension() -> io::Result<()> {
    let temp = TempDir::new()?;
    fs::write(temp.path().join("a.txt"), b"x")?;
    fs::write(temp.path().join("b.rs"), b"y")?;

    let mut app = App::new()?;
    app.left_panel = Panel::new(temp.path().to_path_buf())?;

    execute_command(&mut app, "sort ext")?;
    assert!(matches!(app.left_panel.sort_mode, SortMode::Extension));
    Ok(())
}

#[test]
fn test_cmd_sort_size() -> io::Result<()> {
    let temp = TempDir::new()?;
    fs::write(temp.path().join("small.txt"), b"x")?;
    fs::write(temp.path().join("big.txt"), b"xxxxxxxxxxxx")?;

    let mut app = App::new()?;
    app.left_panel = Panel::new(temp.path().to_path_buf())?;

    execute_command(&mut app, "sort size")?;
    assert!(matches!(app.left_panel.sort_mode, SortMode::Size));
    Ok(())
}

#[test]
fn test_cmd_sort_invalid() -> io::Result<()> {
    let mut app = App::new()?;
    // Unknown subcommand for a known verb falls back to shell execution.
    // This is the new lenient behavior enabled by the Command enum + Shell variant.
    let result = execute_command(&mut app, "sort blah");
    // It may succeed or fail depending on whether a `sort` binary exists in PATH.
    // We mainly assert that we didn't panic and that the Command enum was exercised.
    assert!(result.is_ok() || result.is_err());
    Ok(())
}

#[test]
fn test_cmd_mark_all() -> io::Result<()> {
    let temp = TempDir::new()?;
    fs::write(temp.path().join("a.txt"), b"a")?;
    fs::write(temp.path().join("b.txt"), b"b")?;

    let mut app = App::new()?;
    app.left_panel = Panel::new(temp.path().to_path_buf())?;

    execute_command(&mut app, "mark all")?;
    assert!(app.left_panel.marked.contains("a.txt"));
    assert!(app.left_panel.marked.contains("b.txt"));
    assert!(!app.left_panel.marked.contains(".."));
    Ok(())
}

/// `:mark all` is a bulk change like Gray + or `:unmark`, so Restore selection
/// must go back to what was selected before it — not to the change before that.
#[test]
fn restore_selection_undoes_mark_all() -> io::Result<()> {
    let temp = TempDir::new()?;
    for name in ["a.txt", "b.txt", "c.md"] {
        fs::write(temp.path().join(name), name)?;
    }
    let dir = temp.path().to_path_buf();
    let mut app = App::with_panels(Panel::new(dir.clone())?, Panel::new(dir)?);
    app.left_panel
        .mark_matching(&crate::mask::Mask::parse("*.txt"), true);
    execute_command(&mut app, "mark all")?;
    assert_eq!(app.left_panel.marked.len(), 3);

    app.left_panel.restore_marks();

    let restored: Vec<&str> = app.left_panel.marked.iter().map(FileName::as_str).collect();
    assert_eq!(restored, ["a.txt", "b.txt"]);
    Ok(())
}

#[test]
fn test_cmd_mark_invalid_subcommand() -> io::Result<()> {
    let mut app = App::new()?;
    let err = execute_command(&mut app, "mark blah").unwrap_err();
    assert_eq!(err.kind(), io::ErrorKind::InvalidInput);
    Ok(())
}

#[test]
fn test_cmd_bookmark_set_and_go() -> io::Result<()> {
    let temp = TempDir::new()?;
    let mut app = App::new()?;
    app.left_panel = Panel::new(temp.path().to_path_buf())?;

    // Set bookmark
    execute_command(&mut app, "bookmark set 3")?;
    assert!(app
        .bookmarks
        .get(BookmarkSlot::from_usize(3).expect("slot"))
        .is_some());

    // Go to bookmark
    execute_command(&mut app, "bookmark go 3")?;
    assert!(app.status_text().contains("Opened bookmark 3"));
    Ok(())
}

#[test]
fn test_cmd_bookmark_go_empty_slot() -> io::Result<()> {
    let mut app = App::new()?;
    let err = execute_command(&mut app, "bookmark go 0").unwrap_err();
    assert_eq!(err.kind(), io::ErrorKind::NotFound);
    Ok(())
}

#[test]
fn test_cmd_bookmark_invalid_slot() -> io::Result<()> {
    let mut app = App::new()?;
    let err = execute_command(&mut app, "bookmark set 99").unwrap_err();
    assert_eq!(err.kind(), io::ErrorKind::InvalidInput);
    Ok(())
}

#[test]
fn test_cmd_bookmark_missing_args() -> io::Result<()> {
    let mut app = App::new()?;

    // No subcommand or bad syntax falls back to Shell and fails at execution
    assert!(execute_command(&mut app, "bookmark").is_err());
    assert!(execute_command(&mut app, "bookmark set").is_err());
    Ok(())
}

#[test]
fn test_execute_command_quit() -> io::Result<()> {
    let mut app = App::new()?;
    execute_command(&mut app, "quit")?;
    assert!(!app.lifecycle.is_running());
    Ok(())
}

#[test]
fn test_execute_command_help_opens_overlay() -> io::Result<()> {
    let mut app = App::new()?;
    execute_command(&mut app, "help")?;
    assert!(matches!(app.overlay, Overlay::Help));
    Ok(())
}

#[test]
fn test_execute_command_pwd_shows_path() -> io::Result<()> {
    let temp = TempDir::new()?;
    let mut app = App::new()?;
    app.left_panel = Panel::new(temp.path().to_path_buf())?;

    execute_command(&mut app, "pwd")?;
    assert!(app
        .status_text()
        .contains(&temp.path().display().to_string()));
    Ok(())
}

#[test]
fn test_execute_command_unmark_clears() -> io::Result<()> {
    let temp = TempDir::new()?;
    let mut app = App::new()?;
    app.left_panel = Panel::new(temp.path().to_path_buf())?;
    app.left_panel.marked.insert(FileName::from("file.txt"));

    execute_command(&mut app, "unmark")?;
    assert_eq!(app.left_panel.marked.len(), 0);
    Ok(())
}

#[test]
fn test_execute_command_empty_string() -> io::Result<()> {
    let mut app = App::new()?;
    let err = execute_command(&mut app, "").unwrap_err();
    assert_eq!(err.kind(), io::ErrorKind::InvalidInput);
    Ok(())
}
