//! Purpose: Keys that open overlays or end the session, through the event loop.

use super::*;

const fn key_shift(code: KeyCode) -> KeyEvent {
    KeyEvent::new(code, KeyModifiers::SHIFT)
}

#[test]
fn test_event_loop_f1_help_toggle() -> io::Result<()> {
    use crossterm::event::Event;

    let app = App::new()?;
    run_event_sequence(
        app,
        vec![
            Event::Key(key(KeyCode::F(1))),
            Event::Key(key(KeyCode::F(1))),
            Event::Key(key(KeyCode::F(10))),
            // F10 now asks before quitting; Enter answers Yes.
            Event::Key(key(KeyCode::Enter)),
        ],
    )
}

#[test]
fn test_event_loop_ctrl_u_swap() -> io::Result<()> {
    use crossterm::event::Event;

    let app = App::new()?;
    run_event_sequence(
        app,
        vec![
            Event::Key(key_ctrl(KeyCode::Char('u'))),
            Event::Key(key(KeyCode::F(10))),
            // F10 now asks before quitting; Enter answers Yes.
            Event::Key(key(KeyCode::Enter)),
        ],
    )
}

#[test]
fn test_event_loop_ctrl_o_console() -> io::Result<()> {
    use crossterm::event::Event;

    let app = App::new()?;
    run_event_sequence(
        app,
        vec![
            Event::Key(key_ctrl(KeyCode::Char('o'))),
            Event::Key(key_ctrl(KeyCode::Char('o'))),
            Event::Key(key(KeyCode::F(10))),
            // F10 now asks before quitting; Enter answers Yes.
            Event::Key(key(KeyCode::Enter)),
        ],
    )
}

#[test]
fn test_event_loop_f10_quit() -> io::Result<()> {
    use crossterm::event::Event;

    let app = App::new()?;
    run_event_sequence(
        app,
        vec![
            Event::Key(key(KeyCode::F(10))),
            // F10 now asks before quitting; Enter answers Yes.
            Event::Key(key(KeyCode::Enter)),
        ],
    )
}

#[test]
fn test_event_loop_ctrl_c_quit() -> io::Result<()> {
    use crossterm::event::Event;

    let app = App::new()?;
    run_event_sequence(app, vec![Event::Key(key_ctrl(KeyCode::Char('c')))])
}

#[test]
fn test_event_loop_shift_f4_new_file() -> io::Result<()> {
    use crossterm::event::Event;

    let temp = TempDir::new()?;
    let mut app = App::new()?;
    app.left_panel.path = temp.path().to_path_buf();
    app.left_panel.load_entries()?;

    run_event_sequence(
        app,
        vec![
            Event::Key(key_shift(KeyCode::F(4))),
            Event::Key(key(KeyCode::Char('t'))),
            Event::Key(key(KeyCode::Esc)),
            Event::Key(key(KeyCode::F(10))),
            // F10 now asks before quitting; Enter answers Yes.
            Event::Key(key(KeyCode::Enter)),
        ],
    )
}

#[test]
fn test_event_loop_alt_f1_drive_selection() -> io::Result<()> {
    use crossterm::event::Event;

    let app = App::new()?;
    run_event_sequence(
        app,
        vec![
            Event::Key(KeyEvent::new(KeyCode::F(1), KeyModifiers::ALT)),
            Event::Key(key(KeyCode::Esc)),
            Event::Key(key(KeyCode::F(10))),
            // F10 now asks before quitting; Enter answers Yes.
            Event::Key(key(KeyCode::Enter)),
        ],
    )
}

#[test]
fn test_event_loop_alt_f2_drive_selection() -> io::Result<()> {
    use crossterm::event::Event;

    let app = App::new()?;
    run_event_sequence(
        app,
        vec![
            Event::Key(KeyEvent::new(KeyCode::F(2), KeyModifiers::ALT)),
            Event::Key(key(KeyCode::Esc)),
            Event::Key(key(KeyCode::F(10))),
            // F10 now asks before quitting; Enter answers Yes.
            Event::Key(key(KeyCode::Enter)),
        ],
    )
}

#[test]
fn test_event_loop_f2_bookmarks() -> io::Result<()> {
    use crossterm::event::Event;

    let app = App::new()?;
    run_event_sequence(
        app,
        vec![
            Event::Key(key(KeyCode::F(2))),
            Event::Key(key(KeyCode::Esc)),
            Event::Key(key(KeyCode::F(10))),
            // F10 now asks before quitting; Enter answers Yes.
            Event::Key(key(KeyCode::Enter)),
        ],
    )
}
