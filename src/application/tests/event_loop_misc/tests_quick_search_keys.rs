//! Purpose: quick search keystrokes inside the event loop: fallback, clearing and Backspace.

use super::*;

// -----------------------------------------------------------------------
// Event loop: quick search no match falls back to single char
// -----------------------------------------------------------------------

#[test]
fn test_event_loop_quick_search_no_match_fallback() -> io::Result<()> {
    let temp_dir = TempDir::new()?;
    fs::write(temp_dir.path().join("xyz.txt"), "data")?;

    let mut app = App::new()?;
    app.left_panel = Panel::new(temp_dir.path().to_path_buf())?;
    app.right_panel = Panel::new(temp_dir.path().to_path_buf())?;

    let backend = WritableTestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend)?;

    let mut events = vec![
        // Search for 'x' (matches xyz.txt)
        Event::Key(key_event_with_mods(KeyCode::Char('x'), KeyModifiers::ALT)),
        // Append 'q' -> "xq" won't match, falls back to just 'q' (also no match)
        Event::Key(key_event_with_mods(KeyCode::Char('q'), KeyModifiers::ALT)),
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

// -----------------------------------------------------------------------
// Event loop: unknown key clears quick search
// -----------------------------------------------------------------------

#[test]
fn test_event_loop_unknown_key_clears_quick_search() -> io::Result<()> {
    let temp_dir = TempDir::new()?;
    let mut app = App::new()?;
    app.left_panel = Panel::new(temp_dir.path().to_path_buf())?;
    app.right_panel = Panel::new(temp_dir.path().to_path_buf())?;
    app.quick_search = QuickSearch::Active("test".to_string());

    let backend = WritableTestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend)?;

    let mut events = vec![
        // Null is an unhandled key that should clear quick_search
        // (PageUp/PageDown now page the panel, so they are no longer unmapped).
        Event::Key(key_event(KeyCode::Null)),
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
// Event loop: backspace without quick search is a no-op
// -----------------------------------------------------------------------

#[test]
fn test_event_loop_backspace_without_search() -> io::Result<()> {
    let temp_dir = TempDir::new()?;
    let mut app = App::new()?;
    app.left_panel = Panel::new(temp_dir.path().to_path_buf())?;
    app.right_panel = Panel::new(temp_dir.path().to_path_buf())?;

    let backend = WritableTestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend)?;

    let mut events = vec![
        Event::Key(key_event(KeyCode::Backspace)),
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
// Backspace in quick search pops character
// -----------------------------------------------------------------------

#[test]
fn test_event_loop_backspace_pops_quick_search() -> io::Result<()> {
    let temp_dir = TempDir::new()?;
    fs::write(temp_dir.path().join("abc.txt"), "a")?;

    let mut app = App::new()?;
    app.left_panel = Panel::new(temp_dir.path().to_path_buf())?;
    // Set up quick search with 2 chars so backspace pops one (not clearing)
    app.quick_search = QuickSearch::Active("ab".to_owned());

    let backend = WritableTestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend)?;

    let mut events = vec![
        Event::Key(key_event(KeyCode::Backspace)),
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
