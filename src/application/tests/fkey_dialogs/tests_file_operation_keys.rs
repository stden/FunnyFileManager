//! Purpose: F5/F6/F7/F8 file operations driven through the event loop.

use super::*;

// -----------------------------------------------------------------------
// F5 / F6 / F7 / F8 -- verify the handler functions run without panic
// -----------------------------------------------------------------------

#[test]
fn test_f5_copy_key_in_event_loop() -> io::Result<()> {
    let temp_dir = TempDir::new().expect("test setup");
    let (mut app, _left_dir, right_dir) =
        transfer_fixture(&temp_dir, "left", "right", "copy_me.txt", b"data");

    // Select the file.
    assert!(app.left_panel.select_named("copy_me.txt"));

    let backend = WritableTestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).expect("test setup");

    let mut events = vec![
        // F5 opens the Copy dialog on the other panel's path; Enter accepts it.
        Event::Key(key_event(KeyCode::F(5))),
        Event::Key(key_event(KeyCode::Enter)),
        Event::Key(key_event(KeyCode::F(10))),
        // F10 now asks before quitting; Enter answers Yes.
        Event::Key(key_event(KeyCode::Enter)),
    ]
    .into_iter();

    let result = run_app_with_event_reader(&mut terminal, app, || {
        events
            .next()
            .ok_or_else(|| io::Error::new(io::ErrorKind::UnexpectedEof, "No more events"))
    });

    assert!(result.is_ok());
    assert!(
        right_dir.join("copy_me.txt").exists(),
        "file should have been copied"
    );

    Ok(())
}

#[test]
fn test_f6_move_key_in_event_loop() -> io::Result<()> {
    let temp_dir = TempDir::new().expect("test setup");
    let (mut app, left_dir, right_dir) =
        transfer_fixture(&temp_dir, "left_f6", "right_f6", "move_me.txt", b"move");

    assert!(app.left_panel.select_named("move_me.txt"));

    let backend = WritableTestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).expect("test setup");

    let mut events = vec![
        // F6 opens the Rename/Move dialog; Enter accepts the offered path.
        Event::Key(key_event(KeyCode::F(6))),
        Event::Key(key_event(KeyCode::Enter)),
        Event::Key(key_event(KeyCode::F(10))),
        // F10 now asks before quitting; Enter answers Yes.
        Event::Key(key_event(KeyCode::Enter)),
    ]
    .into_iter();

    let result = run_app_with_event_reader(&mut terminal, app, || {
        events
            .next()
            .ok_or_else(|| io::Error::new(io::ErrorKind::UnexpectedEof, "No more events"))
    });

    assert!(result.is_ok());
    assert!(
        !left_dir.join("move_me.txt").exists(),
        "file should have been moved away"
    );
    assert!(
        right_dir.join("move_me.txt").exists(),
        "file should be at destination"
    );

    Ok(())
}

#[test]
fn test_f7_mkdir_key_in_event_loop() -> io::Result<()> {
    let temp_dir = TempDir::new().expect("test setup");

    let mut app = App::new().expect("test setup");
    app.left_panel = Panel::new(temp_dir.path().to_path_buf()).expect("test setup");
    app.right_panel = Panel::new(temp_dir.path().to_path_buf()).expect("test setup");

    let backend = WritableTestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).expect("test setup");

    let mut events = vec![
        Event::Key(key_event(KeyCode::F(7))),
        // F7 asks for a name now, as Far does.
        Event::Key(key_event(KeyCode::Char('n'))),
        Event::Key(key_event(KeyCode::Char('e'))),
        Event::Key(key_event(KeyCode::Char('w'))),
        Event::Key(key_event(KeyCode::Enter)),
        Event::Key(key_event(KeyCode::F(10))),
        // F10 now asks before quitting; Enter answers Yes.
        Event::Key(key_event(KeyCode::Enter)),
    ]
    .into_iter();

    let result = run_app_with_event_reader(&mut terminal, app, || {
        events
            .next()
            .ok_or_else(|| io::Error::new(io::ErrorKind::UnexpectedEof, "No more events"))
    });

    assert!(result.is_ok());
    assert!(
        temp_dir.path().join("new").exists(),
        "the folder should carry the name that was typed"
    );

    Ok(())
}

#[test]
fn test_f8_delete_confirm_then_y_deletes_file() -> io::Result<()> {
    let temp_dir = TempDir::new().expect("test setup");
    let file_path = temp_dir.path().join("del_via_f8.txt");
    fs::write(&file_path, b"bye").expect("test setup");

    let mut app = App::new().expect("test setup");
    app.left_panel = Panel::new(temp_dir.path().to_path_buf()).expect("test setup");
    app.right_panel = Panel::new(temp_dir.path().to_path_buf()).expect("test setup");
    app.active_panel = PanelSide::Left;

    assert!(app.left_panel.select_named("del_via_f8.txt"));

    let backend = WritableTestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).expect("test setup");

    let mut events = vec![
        // F8 sets up delete_confirm overlay.
        Event::Key(key_event(KeyCode::F(8))),
        // 'y' confirms deletion.
        Event::Key(key_event(KeyCode::Char('y'))),
        Event::Key(key_event(KeyCode::F(10))),
        // F10 now asks before quitting; Enter answers Yes.
        Event::Key(key_event(KeyCode::Enter)),
    ]
    .into_iter();

    let result = run_app_with_event_reader(&mut terminal, app, || {
        events
            .next()
            .ok_or_else(|| io::Error::new(io::ErrorKind::UnexpectedEof, "No more events"))
    });

    assert!(result.is_ok());
    assert!(!file_path.exists(), "file should have been deleted");

    Ok(())
}

fn transfer_fixture(
    root: &TempDir,
    left: &str,
    right: &str,
    name: &str,
    content: &[u8],
) -> (App, PathBuf, PathBuf) {
    let left = root.path().join(left);
    let right = root.path().join(right);
    fs::create_dir(&left).expect("source directory");
    fs::create_dir(&right).expect("destination directory");
    fs::write(left.join(name), content).expect("source file");
    let mut app = App::new().expect("app");
    app.left_panel = Panel::new(left.clone()).expect("source panel");
    app.right_panel = Panel::new(right.clone()).expect("destination panel");
    app.active_panel = PanelSide::Left;
    (app, left, right)
}
