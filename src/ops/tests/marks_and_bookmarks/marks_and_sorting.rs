//! Purpose: Typed mark, unmark, sort and pwd commands.

use super::*;

#[test]
fn test_execute_command_mark_all() -> io::Result<()> {
    let temp_dir = TempDir::new().expect("test setup");
    fs::write(temp_dir.path().join("a.txt"), b"a").expect("test setup");
    fs::write(temp_dir.path().join("b.txt"), b"b").expect("test setup");

    let mut app = App::new().expect("test setup");
    app.left_panel = Panel::new(temp_dir.path().to_path_buf()).expect("test setup");

    execute_command(&mut app, "mark all").expect("test setup");
    assert!(app.left_panel.marked.contains("a.txt"));
    assert!(app.left_panel.marked.contains("b.txt"));
    // ".." should not be marked
    assert!(!app.left_panel.marked.contains(".."));
    assert!(app.status_text().contains("Marked"));

    Ok(())
}

// -----------------------------------------------------------------------
// Bookmarks overlay -- F2 key closes overlay
// -----------------------------------------------------------------------

// -----------------------------------------------------------------------
// Search overlay -- backspace on single character leaves Some("")
// -----------------------------------------------------------------------

// -----------------------------------------------------------------------
// execute_command -- direct unit tests for every command arm
// -----------------------------------------------------------------------

#[test]
fn test_execute_command_pwd_direct() -> io::Result<()> {
    let temp_dir = TempDir::new().expect("test setup");
    let mut app = App::new().expect("test setup");
    app.left_panel = Panel::new(temp_dir.path().to_path_buf()).expect("test setup");

    execute_command(&mut app, "pwd").expect("pwd command");

    assert!(
        app.status_text()
            .contains(&temp_dir.path().display().to_string()),
        "status_message should contain the current panel path"
    );

    Ok(())
}

#[test]
fn test_execute_command_unmark() -> io::Result<()> {
    let temp_dir = TempDir::new().expect("test setup");
    fs::write(temp_dir.path().join("a.txt"), b"a").expect("test setup");

    let mut app = App::new().expect("test setup");
    app.left_panel = Panel::new(temp_dir.path().to_path_buf()).expect("test setup");
    app.left_panel.marked.insert(FileName::from("a.txt"));

    execute_command(&mut app, "unmark").expect("test setup");
    assert!(app.left_panel.marked.is_empty());
    assert!(app.status_text().contains("Unmarked"));

    Ok(())
}

#[test]
fn test_execute_command_sort_size_direct() -> io::Result<()> {
    let temp_dir = TempDir::new().expect("test setup");
    let mut app = App::new().expect("test setup");
    app.left_panel = Panel::new(temp_dir.path().to_path_buf()).expect("test setup");

    execute_command(&mut app, "sort size").expect("sort size");

    assert_eq!(app.get_active_panel().sort_mode, SortMode::Size);

    Ok(())
}

#[test]
fn test_execute_command_sort_name_direct() -> io::Result<()> {
    let temp_dir = TempDir::new().expect("test setup");
    let mut app = App::new().expect("test setup");
    app.left_panel = Panel::new(temp_dir.path().to_path_buf()).expect("test setup");
    app.get_active_panel().sort_mode = SortMode::Size;

    execute_command(&mut app, "sort name").expect("sort name");

    assert_eq!(app.get_active_panel().sort_mode, SortMode::Name);

    Ok(())
}

#[test]
fn test_execute_command_mark_all_direct() -> io::Result<()> {
    let temp_dir = TempDir::new().expect("test setup");
    File::create(temp_dir.path().join("file1.txt")).expect("test setup");
    File::create(temp_dir.path().join("file2.txt")).expect("test setup");

    let mut app = App::new().expect("test setup");
    app.left_panel = Panel::new(temp_dir.path().to_path_buf()).expect("test setup");

    execute_command(&mut app, "mark all").expect("mark all");

    let marked = &app.get_active_panel().marked;
    assert!(!marked.is_empty(), "mark all should mark entries");
    assert!(marked.contains("file1.txt"));
    assert!(marked.contains("file2.txt"));
    // ".." should never be marked
    assert!(!marked.contains(".."));

    Ok(())
}

#[test]
fn test_execute_command_unmark_direct() -> io::Result<()> {
    let temp_dir = TempDir::new().expect("test setup");
    File::create(temp_dir.path().join("file.txt")).expect("test setup");

    let mut app = App::new().expect("test setup");
    app.left_panel = Panel::new(temp_dir.path().to_path_buf()).expect("test setup");
    app.left_panel.marked.insert(FileName::from("file.txt"));
    assert!(!app.left_panel.marked.is_empty());

    execute_command(&mut app, "unmark").expect("unmark");

    assert!(
        app.get_active_panel().marked.is_empty(),
        "unmark should clear all marks"
    );
    assert!(app.status_text().contains("Unmarked"));

    Ok(())
}
