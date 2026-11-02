//! Purpose: cursor movement keys inside the event loop: Home/End, Up/Down and Enter.

use super::*;

// -----------------------------------------------------------------------
// Event loop: Home and End keys
// -----------------------------------------------------------------------

#[test]
fn test_event_loop_home_end_keys() -> io::Result<()> {
    let temp_dir = TempDir::new()?;
    fs::write(temp_dir.path().join("a.txt"), "a")?;
    fs::write(temp_dir.path().join("z.txt"), "z")?;

    let mut app = App::new()?;
    app.left_panel = Panel::new(temp_dir.path().to_path_buf())?;
    app.right_panel = Panel::new(temp_dir.path().to_path_buf())?;

    let backend = WritableTestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend)?;

    let mut events = vec![
        Event::Key(key_event(KeyCode::End)),
        Event::Key(key_event(KeyCode::Home)),
        Event::Key(key_event(KeyCode::F(10))),
        // F10 now asks before quitting; Enter answers Yes.
        Event::Key(key_event(KeyCode::Enter)),
    ]
    .into_iter();

    let result = run_app_with_event_reader(&mut terminal, app, || {
        events
            .next()
            .ok_or_else(|| io::Error::new(io::ErrorKind::UnexpectedEof, "done"))
    });
    assert!(result.is_ok());
    Ok(())
}

// -----------------------------------------------------------------------
// Event loop: Down/Up navigation
// -----------------------------------------------------------------------

#[test]
fn test_event_loop_down_up_navigation() -> io::Result<()> {
    let temp_dir = TempDir::new()?;
    fs::write(temp_dir.path().join("a.txt"), "a")?;
    fs::write(temp_dir.path().join("b.txt"), "b")?;
    let mut app = App::new()?;
    app.left_panel = Panel::new(temp_dir.path().to_path_buf())?;
    app.right_panel = Panel::new(temp_dir.path().to_path_buf())?;

    let backend = WritableTestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend)?;

    let mut events = vec![
        Event::Key(key_event(KeyCode::Down)),
        Event::Key(key_event(KeyCode::Down)),
        Event::Key(key_event(KeyCode::Up)),
        Event::Key(key_event(KeyCode::F(10))),
        // F10 now asks before quitting; Enter answers Yes.
        Event::Key(key_event(KeyCode::Enter)),
    ]
    .into_iter();

    let result = run_app_with_event_reader(&mut terminal, app, || {
        events
            .next()
            .ok_or_else(|| io::Error::new(io::ErrorKind::UnexpectedEof, "done"))
    });
    assert!(result.is_ok());
    Ok(())
}

// -----------------------------------------------------------------------
// Event loop: Enter key
// -----------------------------------------------------------------------

#[test]
fn test_event_loop_enter_key() -> io::Result<()> {
    let temp_dir = TempDir::new()?;
    let sub = temp_dir.path().join("sub");
    fs::create_dir(&sub)?;
    let mut app = App::new()?;
    app.left_panel = Panel::new(temp_dir.path().to_path_buf())?;
    app.right_panel = Panel::new(temp_dir.path().to_path_buf())?;
    // Select the subdirectory
    for (i, entry) in app.left_panel.entries.iter().enumerate() {
        if entry.name == "sub" {
            app.left_panel.select(Some(i));
            break;
        }
    }

    let backend = WritableTestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend)?;

    let mut events = vec![
        Event::Key(key_event(KeyCode::Enter)),
        Event::Key(key_event(KeyCode::F(10))),
        // F10 now asks before quitting; Enter answers Yes.
        Event::Key(key_event(KeyCode::Enter)),
    ]
    .into_iter();

    let result = run_app_with_event_reader(&mut terminal, app, || {
        events
            .next()
            .ok_or_else(|| io::Error::new(io::ErrorKind::UnexpectedEof, "done"))
    });
    assert!(result.is_ok());
    Ok(())
}

// -----------------------------------------------------------------------
// End key moves to last entry
// -----------------------------------------------------------------------

#[test]
fn test_event_loop_end_key_moves_to_last() -> io::Result<()> {
    let temp_dir = TempDir::new()?;
    fs::write(temp_dir.path().join("aaa.txt"), "a")?;
    fs::write(temp_dir.path().join("bbb.txt"), "b")?;
    fs::write(temp_dir.path().join("ccc.txt"), "c")?;

    let mut app = App::new()?;
    app.left_panel = Panel::new(temp_dir.path().to_path_buf())?;
    app.left_panel.select(Some(0));

    let backend = WritableTestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend)?;

    let mut events = vec![
        Event::Key(key_event(KeyCode::End)),
        Event::Key(key_event(KeyCode::F(10))),
        // F10 now asks before quitting; Enter answers Yes.
        Event::Key(key_event(KeyCode::Enter)),
    ]
    .into_iter();

    run_app_with_event_reader(&mut terminal, app, || {
        events
            .next()
            .ok_or_else(|| io::Error::new(io::ErrorKind::UnexpectedEof, "done"))
    })?;

    Ok(())
}
