//! Purpose: F1 toggles help on and off, Shift+F4 opens the new-file dialog through the event loop, and the test backend draws.

use super::*;

// -----------------------------------------------------------------------
// F1 twice brings help up then back down
// -----------------------------------------------------------------------

#[test]
fn test_f1_twice_toggles_help_on_then_off() -> io::Result<()> {
    let temp_dir = TempDir::new().expect("test setup");

    let mut app = App::new().expect("test setup");
    app.left_panel = Panel::new(temp_dir.path().to_path_buf()).expect("test setup");
    app.right_panel = Panel::new(temp_dir.path().to_path_buf()).expect("test setup");

    let backend = WritableTestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).expect("test setup");

    let mut events = vec![
        // F1 toggles help ON.
        Event::Key(key_event(KeyCode::F(1))),
        // F1 again toggles help OFF (the help overlay intercepts it and sets show_help=false).
        Event::Key(key_event(KeyCode::F(1))),
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

// --- WritableTestBackend additional coverage ---

#[test]
fn test_writable_test_backend_draw() -> io::Result<()> {
    let mut backend = WritableTestBackend::new(80, 24);
    backend.hide_cursor().expect("hide cursor");
    backend.show_cursor().expect("show cursor");
    let size = backend.size().expect("size");
    assert_eq!(size.width, 80);
    assert_eq!(size.height, 24);
    Ok(())
}

// -----------------------------------------------------------------------
// Event loop: Alt+F1 / Alt+F2 / Shift+F4 modifier branches
// -----------------------------------------------------------------------

#[test]
fn test_event_loop_shift_f4_opens_new_file_dialog() -> io::Result<()> {
    let temp_dir = TempDir::new()?;
    let mut app = App::new()?;
    app.left_panel = Panel::new(temp_dir.path().to_path_buf())?;
    app.right_panel = Panel::new(temp_dir.path().to_path_buf())?;

    let backend = WritableTestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend)?;

    let mut events = vec![
        Event::Key(key_event_with_mods(KeyCode::F(4), KeyModifiers::SHIFT)),
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
