//! Purpose: Drive selection overlay: navigation, wrap-around and choosing a drive.

use super::*;

// ==================== Overlay: Drive Selection ====================

#[test]
fn test_drive_selection_navigate_and_escape() -> io::Result<()> {
    let temp = TempDir::new().unwrap();
    let (app, mut terminal) = setup_app(&temp)?;

    let events = vec![
        key_mod(KeyCode::F(1), KeyModifiers::ALT), // open drive selection for left
        key(KeyCode::Down),                        // navigate down
        key(KeyCode::Up),                          // navigate up
        key(KeyCode::Home),                        // go to first
        key(KeyCode::End),                         // go to last
        key(KeyCode::Esc),                         // close
        key(KeyCode::F(10)),
        // F10 now asks before quitting; Enter answers Yes.
        key(KeyCode::Enter),
    ];

    run_events(&mut terminal, app, events)
}

#[test]
fn test_drive_selection_enter_local() -> io::Result<()> {
    let temp = TempDir::new().unwrap();
    let (app, mut terminal) = setup_app(&temp)?;

    let events = vec![
        key_mod(KeyCode::F(1), KeyModifiers::ALT), // open for left
        key(KeyCode::Enter),                       // select first drive (/)
        key(KeyCode::F(10)),
        // F10 now asks before quitting; Enter answers Yes.
        key(KeyCode::Enter),
    ];

    run_events(&mut terminal, app, events)
}

#[test]
fn test_drive_selection_right_panel() -> io::Result<()> {
    let temp = TempDir::new().unwrap();
    let (app, mut terminal) = setup_app(&temp)?;

    let events = vec![
        key_mod(KeyCode::F(2), KeyModifiers::ALT), // open for right
        key(KeyCode::Esc),
        key(KeyCode::F(10)),
        // F10 now asks before quitting; Enter answers Yes.
        key(KeyCode::Enter),
    ];

    run_events(&mut terminal, app, events)
}

#[test]
fn test_drive_selection_up_wraps() -> io::Result<()> {
    let temp = TempDir::new().unwrap();
    let (app, mut terminal) = setup_app(&temp)?;

    let events = vec![
        key_mod(KeyCode::F(1), KeyModifiers::ALT),
        key(KeyCode::Up), // at index 0, should wrap to last
        key(KeyCode::Esc),
        key(KeyCode::F(10)),
        // F10 now asks before quitting; Enter answers Yes.
        key(KeyCode::Enter),
    ];

    run_events(&mut terminal, app, events)
}

#[test]
fn test_drive_selection_down_wraps() -> io::Result<()> {
    let temp = TempDir::new().unwrap();
    let (app, mut terminal) = setup_app(&temp)?;

    let events = vec![
        key_mod(KeyCode::F(1), KeyModifiers::ALT),
        key(KeyCode::End),  // go to last
        key(KeyCode::Down), // should wrap to first
        key(KeyCode::Esc),
        key(KeyCode::F(10)),
        // F10 now asks before quitting; Enter answers Yes.
        key(KeyCode::Enter),
    ];

    run_events(&mut terminal, app, events)
}
