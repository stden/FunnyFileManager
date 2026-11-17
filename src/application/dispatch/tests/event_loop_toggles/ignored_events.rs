//! Purpose: key releases and non-key events pass through the event loop untouched.

use super::*;

// -----------------------------------------------------------------------
// KeyEventKind::Release -- ignored (continue branch)
// -----------------------------------------------------------------------

#[test]
fn test_run_app_release_event_ignored() -> io::Result<()> {
    let temp_dir = TempDir::new().expect("test setup");

    let mut app = App::new().expect("test setup");
    app.left_panel = Panel::new(temp_dir.path().to_path_buf()).expect("test setup");
    app.right_panel = Panel::new(temp_dir.path().to_path_buf()).expect("test setup");

    let backend = WritableTestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).expect("test setup");

    // Build a Release key event (should be silently ignored, not quit)
    let release_event = KeyEvent {
        code: KeyCode::F(10),
        modifiers: KeyModifiers::empty(),
        kind: KeyEventKind::Release,
        state: crossterm::event::KeyEventState::empty(),
    };

    let mut events = vec![
        // Release event -- must be ignored (no quit, no action)
        Event::Key(release_event),
        // Real Press quit event
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

// -----------------------------------------------------------------------
// Non-Key events (mouse, resize) -- hit the outer if-let guard
// -----------------------------------------------------------------------

#[test]
fn test_run_app_non_key_event_ignored() -> io::Result<()> {
    let temp_dir = TempDir::new().expect("test setup");

    let mut app = App::new().expect("test setup");
    app.left_panel = Panel::new(temp_dir.path().to_path_buf()).expect("test setup");
    app.right_panel = Panel::new(temp_dir.path().to_path_buf()).expect("test setup");

    let backend = WritableTestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).expect("test setup");

    let mut events = vec![
        // Resize event -- not a Key event, must fall through silently
        Event::Resize(80, 24),
        Event::Paste("hi".into()),
        Event::FocusGained,
        // Quit
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

// --- Key Release events should be ignored ---

#[test]
fn test_key_release_ignored() -> io::Result<()> {
    let temp_dir = TempDir::new().expect("test setup");
    let mut app = App::new().expect("test setup");
    app.left_panel = Panel::new(temp_dir.path().to_path_buf()).expect("test setup");
    app.right_panel = Panel::new(temp_dir.path().to_path_buf()).expect("test setup");

    let backend = WritableTestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).expect("test setup");

    let release_event = Event::Key(KeyEvent {
        code: KeyCode::Down,
        modifiers: KeyModifiers::empty(),
        kind: KeyEventKind::Release,
        state: crossterm::event::KeyEventState::NONE,
    });

    let mut events = vec![
        release_event,
        // F10 now asks before quitting; Enter answers Yes.
        Event::Key(key_event(KeyCode::F(10))),
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
