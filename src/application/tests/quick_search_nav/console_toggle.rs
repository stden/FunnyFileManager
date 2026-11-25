//! Purpose: Ctrl+O console-mode toggling and the prompt it renders.

use super::*;

// -----------------------------------------------------------------------
// Ctrl+O -- console mode toggle
// -----------------------------------------------------------------------

#[test]
fn test_run_app_ctrl_o_console_toggle() -> io::Result<()> {
    let temp_dir = TempDir::new().expect("test setup");

    let mut app = App::new().expect("test setup");
    app.left_panel = Panel::new(temp_dir.path().to_path_buf()).expect("test setup");
    app.right_panel = Panel::new(temp_dir.path().to_path_buf()).expect("test setup");

    let backend = WritableTestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).expect("test setup");

    let mut events = vec![
        // Ctrl+O -- toggle console mode ON
        Event::Key(key_event_with_mods(
            KeyCode::Char('o'),
            KeyModifiers::CONTROL,
        )),
        // Ctrl+O -- toggle console mode OFF
        Event::Key(key_event_with_mods(
            KeyCode::Char('o'),
            KeyModifiers::CONTROL,
        )),
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
fn test_run_app_ctrl_o_renders_console_prompt() -> io::Result<()> {
    let temp_dir = TempDir::new().expect("test setup");
    let mut app = App::new().expect("test setup");
    app.left_panel = Panel::new(temp_dir.path().to_path_buf()).expect("test setup");
    app.right_panel = Panel::new(temp_dir.path().to_path_buf()).expect("test setup");

    let backend = WritableTestBackend::new(120, 24);
    let mut terminal = Terminal::new(backend).expect("test setup");

    let mut events = vec![Event::Key(key_event_with_mods(
        KeyCode::Char('o'),
        KeyModifiers::CONTROL,
    ))]
    .into_iter();

    let err = run_app_with_event_reader(&mut terminal, app, || {
        events.next().ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::UnexpectedEof,
                "stop after console mode renders",
            )
        })
    })
    .expect_err("event reader should stop after rendering console mode");

    assert_eq!(err.kind(), io::ErrorKind::UnexpectedEof);
    let screen = buffer_text(&terminal);
    // Nothing has been run yet, so this is a fresh terminal: a prompt on the
    // top row, and no panel chrome anywhere.
    assert!(screen.starts_with('●'), "{screen}");
    assert!(!screen.contains('╔'), "panels still drawn: {screen}");
    Ok(())
}
