//! Purpose: ':' opens the command line from the main view and renders its prompt.

use super::*;

// --- ':' opens command line ---

#[test]
fn test_colon_opens_command_line() -> io::Result<()> {
    let temp_dir = TempDir::new().expect("test setup");
    let mut app = App::new().expect("test setup");
    app.left_panel = Panel::new(temp_dir.path().to_path_buf()).expect("test setup");
    app.right_panel = Panel::new(temp_dir.path().to_path_buf()).expect("test setup");

    let backend = WritableTestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).expect("test setup");

    let mut events = vec![
        // Open command line
        Event::Key(key_event(KeyCode::Char(':'))),
        // Cancel
        Event::Key(key_event(KeyCode::Esc)),
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
fn test_run_app_colon_renders_command_prompt() -> io::Result<()> {
    let temp_dir = TempDir::new().expect("test setup");
    let mut app = App::new().expect("test setup");
    app.left_panel = Panel::new(temp_dir.path().to_path_buf()).expect("test setup");
    app.right_panel = Panel::new(temp_dir.path().to_path_buf()).expect("test setup");

    let backend = WritableTestBackend::new(160, 24);
    let mut terminal = Terminal::new(backend).expect("test setup");

    let mut events = vec![Event::Key(key_event(KeyCode::Char(':')))].into_iter();

    let err = run_app_with_event_reader(&mut terminal, app, || {
        events.next().ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::UnexpectedEof,
                "stop after command line opens",
            )
        })
    })
    .expect_err("event reader should stop after rendering command line");

    assert_eq!(err.kind(), io::ErrorKind::UnexpectedEof);
    let screen = buffer_text(&terminal);
    // The caret is a reversed cell rather than a `_` glyph now, so the
    // prompt itself is what the rendered screen can be asserted on.
    assert!(screen.contains("# ") || screen.contains("$ "), "{screen}");
    Ok(())
}
