//! Purpose: Copying and moving files and directories to the other panel.

use super::*;

// ==================== handle_copy / handle_move ====================

#[test]
fn test_copy_file_to_other_panel() -> io::Result<()> {
    let temp = TempDir::new().unwrap();
    let left = temp.path().join("left");
    let right = temp.path().join("right");
    fs::create_dir(&left).unwrap();
    fs::create_dir(&right).unwrap();
    fs::write(left.join("source.txt"), b"data").unwrap();

    let mut app = App::new().unwrap();
    app.left_panel = Panel::new(left.clone()).unwrap();
    app.right_panel = Panel::new(right.clone()).unwrap();
    let idx = app
        .left_panel
        .entries
        .iter()
        .position(|e| e.name == "source.txt")
        .unwrap();
    app.left_panel.select(Some(idx));
    let backend = WritableTestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).unwrap();

    let events = vec![
        // F5 opens the Copy dialog; Enter accepts the offered destination.
        key(KeyCode::F(5)),
        key(KeyCode::Enter),
        key(KeyCode::F(10)),
        key(KeyCode::Enter),
    ];

    run_events(&mut terminal, app, events)?;
    assert!(right.join("source.txt").exists());
    assert!(left.join("source.txt").exists()); // original still there
    Ok(())
}

#[test]
fn test_move_file_to_other_panel() -> io::Result<()> {
    let temp = TempDir::new().unwrap();
    let left = temp.path().join("left");
    let right = temp.path().join("right");
    fs::create_dir(&left).unwrap();
    fs::create_dir(&right).unwrap();
    fs::write(left.join("moveme.txt"), b"data").unwrap();

    let mut app = App::new().unwrap();
    app.left_panel = Panel::new(left.clone()).unwrap();
    app.right_panel = Panel::new(right.clone()).unwrap();
    let idx = app
        .left_panel
        .entries
        .iter()
        .position(|e| e.name == "moveme.txt")
        .unwrap();
    app.left_panel.select(Some(idx));
    let backend = WritableTestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).unwrap();

    let events = vec![
        // F6 opens the Rename/Move dialog; Enter accepts the offered path.
        key(KeyCode::F(6)),
        key(KeyCode::Enter),
        key(KeyCode::F(10)),
        key(KeyCode::Enter),
    ];

    run_events(&mut terminal, app, events)?;
    assert!(right.join("moveme.txt").exists());
    assert!(!left.join("moveme.txt").exists()); // original gone
    Ok(())
}

#[test]
fn test_copy_directory() -> io::Result<()> {
    let temp = TempDir::new().unwrap();
    let left = temp.path().join("left");
    let right = temp.path().join("right");
    fs::create_dir(&left).unwrap();
    fs::create_dir(&right).unwrap();
    let sub = left.join("mydir");
    fs::create_dir(&sub).unwrap();
    fs::write(sub.join("inside.txt"), b"data").unwrap();

    let mut app = App::new().unwrap();
    app.left_panel = Panel::new(left).unwrap();
    app.right_panel = Panel::new(right.clone()).unwrap();
    let idx = app
        .left_panel
        .entries
        .iter()
        .position(|e| e.name == "mydir")
        .unwrap();
    app.left_panel.select(Some(idx));
    let backend = WritableTestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).unwrap();

    let events = vec![
        // F5 opens the Copy dialog; Enter accepts the offered destination.
        key(KeyCode::F(5)),
        key(KeyCode::Enter),
        key(KeyCode::F(10)),
        key(KeyCode::Enter),
    ];

    run_events(&mut terminal, app, events)?;
    assert!(right.join("mydir").join("inside.txt").exists());
    Ok(())
}
