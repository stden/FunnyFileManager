//! Purpose: Keys through the event loop: console mode, swap, root, Home/End, marking.

use super::*;

const fn key_release(code: KeyCode) -> Event {
    Event::Key(KeyEvent {
        code,
        modifiers: KeyModifiers::empty(),
        kind: KeyEventKind::Release,
        state: KeyEventState::empty(),
    })
}

// ==================== Key Release Ignored ====================

#[test]
fn test_key_release_ignored() -> io::Result<()> {
    let temp = TempDir::new().unwrap();
    let (app, mut terminal) = setup_app(&temp)?;

    let events = vec![
        key_release(KeyCode::Down), // should be ignored
        key(KeyCode::F(10)),        // quit
        // F10 now asks before quitting; Enter answers Yes.
        key(KeyCode::Enter),
    ];

    run_events(&mut terminal, app, events)
}

// ==================== Console Mode (Ctrl+O) ====================

#[test]
fn test_console_mode_toggle() -> io::Result<()> {
    let temp = TempDir::new().unwrap();
    let (app, mut terminal) = setup_app(&temp)?;

    let events = vec![
        key_mod(KeyCode::Char('o'), KeyModifiers::CONTROL), // enter console mode
        key_mod(KeyCode::Char('o'), KeyModifiers::CONTROL), // exit console mode
        key(KeyCode::F(10)),
        // F10 now asks before quitting; Enter answers Yes.
        key(KeyCode::Enter),
    ];

    run_events(&mut terminal, app, events)
}

// ==================== Ctrl+U Swap Panels ====================

#[test]
fn test_ctrl_u_swap_panels_via_event_loop() -> io::Result<()> {
    let temp = TempDir::new().unwrap();
    let left = temp.path().join("left");
    let right = temp.path().join("right");
    fs::create_dir(&left).unwrap();
    fs::create_dir(&right).unwrap();

    let mut app = App::new().unwrap();
    app.left_panel = Panel::new(left).unwrap();
    app.right_panel = Panel::new(right).unwrap();
    let backend = WritableTestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).unwrap();

    let events = vec![
        key_mod(KeyCode::Char('u'), KeyModifiers::CONTROL),
        key(KeyCode::F(10)),
        // F10 now asks before quitting; Enter answers Yes.
        key(KeyCode::Enter),
    ];

    run_events(&mut terminal, app, events)
}

// ==================== Ctrl+\ Go to Root ====================

#[test]
fn test_ctrl_backslash_goes_to_root() -> io::Result<()> {
    let temp = TempDir::new().unwrap();
    let (app, mut terminal) = setup_app(&temp)?;

    let events = vec![
        key_mod(KeyCode::Char('\\'), KeyModifiers::CONTROL),
        key(KeyCode::F(10)),
        // F10 now asks before quitting; Enter answers Yes.
        key(KeyCode::Enter),
    ];

    run_events(&mut terminal, app, events)
}

// ==================== Home and End Keys ====================

#[test]
fn test_home_end_keys() -> io::Result<()> {
    let temp = TempDir::new().unwrap();
    for i in 0..10 {
        fs::write(temp.path().join(format!("file{i:02}.txt")), b"").unwrap();
    }

    let (app, mut terminal) = setup_app(&temp)?;

    let events = vec![
        key(KeyCode::End),  // go to last
        key(KeyCode::Home), // go to first
        key(KeyCode::F(10)),
        // F10 now asks before quitting; Enter answers Yes.
        key(KeyCode::Enter),
    ];

    run_events(&mut terminal, app, events)
}

// ==================== Insert and Space for Marking ====================

#[test]
fn test_insert_and_space_marking() -> io::Result<()> {
    let temp = TempDir::new().unwrap();
    fs::write(temp.path().join("a.txt"), b"").unwrap();
    fs::write(temp.path().join("b.txt"), b"").unwrap();

    let (app, mut terminal) = setup_app(&temp)?;

    let events = vec![
        key(KeyCode::Down),      // move past ".."
        key(KeyCode::Insert),    // mark and move down
        key(KeyCode::Char(' ')), // mark and move down
        key(KeyCode::F(10)),
        // F10 now asks before quitting; Enter answers Yes.
        key(KeyCode::Enter),
    ];

    run_events(&mut terminal, app, events)
}
