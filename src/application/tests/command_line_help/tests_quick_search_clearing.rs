//! Purpose: quick search clearing keys: Backspace, Esc and Alt+letter with Backspace.

use super::*;

// --- Backspace with no quick search ---

#[test]
fn test_backspace_no_quick_search() -> io::Result<()> {
    let temp_dir = TempDir::new().expect("test setup");
    let mut app = App::new().expect("test setup");
    app.left_panel = Panel::new(temp_dir.path().to_path_buf()).expect("test setup");
    app.right_panel = Panel::new(temp_dir.path().to_path_buf()).expect("test setup");

    let backend = WritableTestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).expect("test setup");

    let mut events = vec![
        // Backspace with no active quick search
        Event::Key(key_event(KeyCode::Backspace)),
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

// --- Esc clears quick search ---

#[test]
fn test_esc_clears_quick_search() -> io::Result<()> {
    let temp_dir = TempDir::new().expect("test setup");
    fs::write(temp_dir.path().join("file.txt"), b"data").expect("test setup");
    fs::write(temp_dir.path().join("foo.txt"), b"f").expect("test setup");

    let mut app = App::new().expect("test setup");
    app.left_panel = Panel::new(temp_dir.path().to_path_buf()).expect("test setup");
    app.right_panel = Panel::new(temp_dir.path().to_path_buf()).expect("test setup");

    let backend = WritableTestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).expect("test setup");

    let mut events = vec![
        // Start quick search
        Event::Key(key_event_with_mods(KeyCode::Char('f'), KeyModifiers::ALT)),
        // Esc clears quick_search (no overlay is active at this point)
        Event::Key(key_event(KeyCode::Esc)),
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
// Event loop: Alt+letter quick search with backspace
// -----------------------------------------------------------------------

#[test]
fn test_event_loop_quick_search_and_backspace() -> io::Result<()> {
    let temp_dir = TempDir::new()?;
    fs::write(temp_dir.path().join("alpha.txt"), "a")?;
    fs::write(temp_dir.path().join("beta.txt"), "b")?;

    let mut app = App::new()?;
    app.left_panel = Panel::new(temp_dir.path().to_path_buf())?;
    app.right_panel = Panel::new(temp_dir.path().to_path_buf())?;

    let backend = WritableTestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend)?;

    let mut events = vec![
        // Start quick search with Alt+a
        Event::Key(key_event_with_mods(KeyCode::Char('a'), KeyModifiers::ALT)),
        // Append another letter
        Event::Key(key_event_with_mods(KeyCode::Char('l'), KeyModifiers::ALT)),
        // Backspace to shorten search
        Event::Key(key_event(KeyCode::Backspace)),
        // Backspace again to clear search
        Event::Key(key_event(KeyCode::Backspace)),
        // Esc clears quick search (already None, tests else branch)
        Event::Key(key_event(KeyCode::Esc)),
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
