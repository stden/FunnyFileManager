//! Purpose: Ctrl+R re-reads both panels and reports success or failure.

use super::*;

// --- Ctrl+R refresh panels ---

#[test]
fn test_ctrl_r_refresh() -> io::Result<()> {
    let temp_dir = TempDir::new().expect("test setup");
    let mut app = App::new().expect("test setup");
    app.left_panel = Panel::new(temp_dir.path().to_path_buf()).expect("test setup");
    app.right_panel = Panel::new(temp_dir.path().to_path_buf()).expect("test setup");

    let backend = WritableTestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).expect("test setup");

    let mut events = vec![
        Event::Key(key_event_with_mods(
            KeyCode::Char('r'),
            KeyModifiers::CONTROL,
        )),
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
    Ok(())
}

#[test]
fn test_ctrl_r_refresh_renders_success_status() -> io::Result<()> {
    let temp_dir = TempDir::new().expect("test setup");
    let mut app = App::new().expect("test setup");
    app.left_panel = Panel::new(temp_dir.path().to_path_buf()).expect("test setup");
    app.right_panel = Panel::new(temp_dir.path().to_path_buf()).expect("test setup");

    let backend = WritableTestBackend::new(240, 24);
    let mut terminal = Terminal::new(backend).expect("test setup");

    let mut events = vec![
        Event::Key(key_event_with_mods(
            KeyCode::Char('r'),
            KeyModifiers::CONTROL,
        )),
        Event::Key(key_event(KeyCode::F(10))),
        // F10 now asks before quitting; Enter answers Yes.
        Event::Key(key_event(KeyCode::Enter)),
    ]
    .into_iter();

    run_app_with_event_reader(&mut terminal, app, || {
        events
            .next()
            .ok_or_else(|| io::Error::new(io::ErrorKind::UnexpectedEof, "No more events"))
    })?;

    let screen = buffer_text(&terminal);
    assert!(screen.contains("Panels refreshed"), "{screen}");
    Ok(())
}

#[test]
fn test_ctrl_r_refresh_renders_failure_status() -> io::Result<()> {
    let temp_dir = TempDir::new().expect("test setup");
    let left_dir = temp_dir.path().join("left");
    let right_dir = temp_dir.path().join("right");
    fs::create_dir(&left_dir).expect("test setup");
    fs::create_dir(&right_dir).expect("test setup");

    let mut app = App::new().expect("test setup");
    app.left_panel = Panel::new(left_dir.clone()).expect("test setup");
    app.right_panel = Panel::new(right_dir).expect("test setup");

    fs::remove_dir_all(&left_dir).expect("test setup");

    let backend = WritableTestBackend::new(240, 24);
    let mut terminal = Terminal::new(backend).expect("test setup");

    let mut events = vec![
        Event::Key(key_event_with_mods(
            KeyCode::Char('r'),
            KeyModifiers::CONTROL,
        )),
        Event::Key(key_event(KeyCode::F(10))),
        // F10 now asks before quitting; Enter answers Yes.
        Event::Key(key_event(KeyCode::Enter)),
    ]
    .into_iter();

    run_app_with_event_reader(&mut terminal, app, || {
        events
            .next()
            .ok_or_else(|| io::Error::new(io::ErrorKind::UnexpectedEof, "No more events"))
    })?;

    let screen = buffer_text(&terminal);
    assert!(screen.contains("Refresh failed"), "{screen}");
    Ok(())
}
