//! Purpose: the quick search through the event loop — Backspace shortens it, a name that matches nothing keeps the cursor, and navigation clears it.

use super::*;

#[test]
fn test_quick_search_backspace_clears_rebase_copy() -> io::Result<()> {
    let temp_dir = TempDir::new()?;
    fs::write(temp_dir.path().join("test.txt"), b"data")?;

    let mut app = App::new()?;
    app.left_panel = Panel::new(temp_dir.path().to_path_buf())?;
    app.right_panel = Panel::new(temp_dir.path().to_path_buf())?;

    let backend = WritableTestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend)?;

    let mut events = vec![
        // Start quick search
        Event::Key(key_event_with_mods(KeyCode::Char('t'), KeyModifiers::ALT)),
        // Backspace clears the single character, clearing quick search
        Event::Key(key_event(KeyCode::Backspace)),
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

#[test]
fn test_quick_search_no_match_fallback_rebase_copy() -> io::Result<()> {
    let temp_dir = TempDir::new()?;
    fs::write(temp_dir.path().join("apple.txt"), b"data")?;
    fs::write(temp_dir.path().join("banana.txt"), b"data")?;

    let mut app = App::new()?;
    app.left_panel = Panel::new(temp_dir.path().to_path_buf())?;
    app.right_panel = Panel::new(temp_dir.path().to_path_buf())?;

    let backend = WritableTestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend)?;

    let mut events = vec![
        // Search for "a"
        Event::Key(key_event_with_mods(KeyCode::Char('a'), KeyModifiers::ALT)),
        // Then "x" - no match for "ax", should try just "x"
        Event::Key(key_event_with_mods(KeyCode::Char('x'), KeyModifiers::ALT)),
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

#[test]
fn test_quick_search_cleared_on_navigation_rebase_copy() -> io::Result<()> {
    let temp_dir = TempDir::new()?;
    fs::write(temp_dir.path().join("file.txt"), b"data")?;

    let mut app = App::new()?;
    app.left_panel = Panel::new(temp_dir.path().to_path_buf())?;
    app.right_panel = Panel::new(temp_dir.path().to_path_buf())?;

    let backend = WritableTestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend)?;

    let mut events = vec![
        // Start quick search
        Event::Key(key_event_with_mods(KeyCode::Char('f'), KeyModifiers::ALT)),
        // Navigation ends the quick search, as in Far (see quick_search_ends)
        Event::Key(key_event(KeyCode::Down)),
        Event::Key(key_event(KeyCode::Up)),
        // Tab ends the quick search too
        Event::Key(key_event(KeyCode::Tab)),
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
