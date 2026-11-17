//! Purpose: Ctrl+backslash, Ctrl+U, Ctrl+O and Ctrl+R keys driven through the event loop.

use super::*;

// -----------------------------------------------------------------------
// Event loop: Insert and Space mark + advance
// -----------------------------------------------------------------------

// -----------------------------------------------------------------------
// Event loop: Ctrl+\ go to root
// -----------------------------------------------------------------------

#[test]
fn test_event_loop_ctrl_backslash_goes_to_root() -> io::Result<()> {
    let temp_dir = TempDir::new()?;
    let mut app = App::new()?;
    app.left_panel = Panel::new(temp_dir.path().to_path_buf())?;
    app.right_panel = Panel::new(temp_dir.path().to_path_buf())?;

    let backend = WritableTestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend)?;

    let mut events = vec![
        Event::Key(key_event_with_mods(
            KeyCode::Char('\\'),
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
            .ok_or_else(|| io::Error::new(io::ErrorKind::UnexpectedEof, "done"))
    });
    assert!(result.is_ok());
    Ok(())
}

// -----------------------------------------------------------------------
// Event loop: Ctrl+U swap, Ctrl+O console, Ctrl+R refresh
// -----------------------------------------------------------------------

#[test]
fn test_event_loop_ctrl_u_swap_panels() -> io::Result<()> {
    let temp_dir = TempDir::new()?;
    let left_dir = temp_dir.path().join("left");
    let right_dir = temp_dir.path().join("right");
    fs::create_dir(&left_dir)?;
    fs::create_dir(&right_dir)?;

    let mut app = App::new()?;
    app.left_panel = Panel::new(left_dir)?;
    app.right_panel = Panel::new(right_dir)?;

    let backend = WritableTestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend)?;

    let mut events = vec![
        Event::Key(key_event_with_mods(
            KeyCode::Char('u'),
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
            .ok_or_else(|| io::Error::new(io::ErrorKind::UnexpectedEof, "done"))
    });
    assert!(result.is_ok());
    Ok(())
}

#[test]
fn test_event_loop_ctrl_o_toggle_console() -> io::Result<()> {
    let temp_dir = TempDir::new()?;
    let mut app = App::new()?;
    app.left_panel = Panel::new(temp_dir.path().to_path_buf())?;
    app.right_panel = Panel::new(temp_dir.path().to_path_buf())?;

    let backend = WritableTestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend)?;

    let mut events = vec![
        Event::Key(key_event_with_mods(
            KeyCode::Char('o'),
            KeyModifiers::CONTROL,
        )),
        Event::Key(key_event_with_mods(
            KeyCode::Char('o'),
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
            .ok_or_else(|| io::Error::new(io::ErrorKind::UnexpectedEof, "done"))
    });
    assert!(result.is_ok());
    Ok(())
}

#[test]
fn test_event_loop_ctrl_r_refresh() -> io::Result<()> {
    let temp_dir = TempDir::new()?;
    let mut app = App::new()?;
    app.left_panel = Panel::new(temp_dir.path().to_path_buf())?;
    app.right_panel = Panel::new(temp_dir.path().to_path_buf())?;

    let backend = WritableTestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend)?;

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
            .ok_or_else(|| io::Error::new(io::ErrorKind::UnexpectedEof, "done"))
    });
    assert!(result.is_ok());
    Ok(())
}
