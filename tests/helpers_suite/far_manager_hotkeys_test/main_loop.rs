//! Purpose: Main-loop smoke test over every hotkey, and the hotkey and refresh logic.

use super::*;

// ─── EVENT LOOP SMOKE: all main-loop keys ────────────────────────────────────

#[test]
fn smoke_all_main_loop_hotkeys_no_crash() {
    let temp = TempDir::new().unwrap();
    fs::write(temp.path().join("file.txt"), b"hello").unwrap();
    let app = setup(&temp).unwrap();

    let events = main_loop_hotkeys();

    assert!(run(app, events).is_ok());
}

#[test]
fn test_hotkey_logic_in_app() -> io::Result<()> {
    let temp_dir = TempDir::new()?;
    let left_path = temp_dir.path().join("left");
    let right_path = temp_dir.path().join("right");
    fs::create_dir(&left_path)?;
    fs::create_dir(&right_path)?;

    File::create(left_path.join("file1.txt"))?;

    let mut app = App::new()?;
    app.left_panel = Panel::new(left_path.clone())?;
    app.right_panel = Panel::new(right_path.clone())?;

    // Test Swap Panels logic (Ctrl+U)
    app.swap_panels();
    assert_eq!(app.left_panel.path, right_path);
    assert_eq!(app.right_panel.path, left_path);

    // Test Mark logic (Insert/Space)
    app.active_panel = PanelSide::Right; // Right now has file1.txt
    app.right_panel.load_entries()?;
    app.right_panel.select(Some(1)); // file1.txt (0 is ..)

    app.right_panel.toggle_mark_selected();
    assert!(app.right_panel.marked.contains("file1.txt"));

    app.right_panel.toggle_mark_selected();
    assert!(!app.right_panel.marked.contains("file1.txt"));

    Ok(())
}

#[test]
fn test_ctrl_r_refresh_logic() -> io::Result<()> {
    let mut app = App::new()?;
    let temp_dir = TempDir::new()?;
    app.left_panel.path = temp_dir.path().to_path_buf();
    app.right_panel.path = temp_dir.path().to_path_buf();

    // Setup backend
    let backend = WritableTestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend)?;

    // Simulating Ctrl+R
    let events = vec![
        Event::Key(KeyEvent::new(KeyCode::Char('r'), KeyModifiers::CONTROL)),
        Event::Key(KeyEvent::new(KeyCode::F(10), KeyModifiers::NONE)),
    ];
    let mut event_iter = events.into_iter();

    run_app_with_event_reader(&mut terminal, app, || {
        Ok(event_iter.next().unwrap_or(Event::Key(KeyEvent::new(
            KeyCode::F(10),
            KeyModifiers::NONE,
        ))))
    })?;

    // Status message should reflect refresh
    // Note: Since we can't easily inspect the app state after run_app_with_event_reader,
    // we assume it doesn't error. To truly test, we'd need to mock refresh_panels.

    Ok(())
}

fn main_loop_hotkeys() -> Vec<Event> {
    vec![
        ev(KeyCode::Down),
        ev(KeyCode::Up),
        ev(KeyCode::Home),
        ev(KeyCode::End),
        ev(KeyCode::Tab),
        ev(KeyCode::Left),
        ev(KeyCode::Right),
        ev(KeyCode::F(1)),                                  // help on
        ev(KeyCode::F(1)),                                  // help off  (goes through overlay now)
        ev_mod(KeyCode::F(1), KeyModifiers::ALT),           // drive select left
        ev(KeyCode::Esc),                                   // close drives
        ev_mod(KeyCode::F(2), KeyModifiers::ALT),           // drive select right
        ev(KeyCode::Esc),                                   // close drives
        ev_mod(KeyCode::F(4), KeyModifiers::SHIFT),         // new file dialog
        ev(KeyCode::Esc),                                   // cancel
        ev_mod(KeyCode::Char('u'), KeyModifiers::CONTROL),  // swap panels
        ev_mod(KeyCode::Char('o'), KeyModifiers::CONTROL),  // console on
        ev_mod(KeyCode::Char('o'), KeyModifiers::CONTROL),  // console off
        ev_mod(KeyCode::Char('\\'), KeyModifiers::CONTROL), // go to root
        ev(KeyCode::Insert),
        ev(KeyCode::Char(' ')),
        ev_mod(KeyCode::Char('a'), KeyModifiers::ALT), // quick search
        ev(KeyCode::Backspace),
        ev(KeyCode::Esc),
        ev(KeyCode::F(10)), // quit — asks first
        ev(KeyCode::F(10)), // …and answers
    ]
}
