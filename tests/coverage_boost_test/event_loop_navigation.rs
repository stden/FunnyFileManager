//! Purpose: Cursor and directory keys through the event loop.

use super::*;

#[test]
fn test_event_loop_tab_switch() -> io::Result<()> {
    let temp = TempDir::new()?;
    let app = setup_app(&temp)?;
    let mut terminal = setup_terminal()?;
    let events = vec![
        key(KeyCode::Tab),
        key(KeyCode::Tab),
        key(KeyCode::F(10)),
        key(KeyCode::Enter),
    ];
    run_events(&mut terminal, app, events);
    Ok(())
}

#[test]
fn test_event_loop_home_end() -> io::Result<()> {
    let temp = TempDir::new()?;
    fs::write(temp.path().join("a.txt"), "a")?;
    fs::write(temp.path().join("b.txt"), "b")?;
    let app = setup_app(&temp)?;
    let mut terminal = setup_terminal()?;
    let events = vec![
        key(KeyCode::End),
        key(KeyCode::Home),
        key(KeyCode::F(10)),
        key(KeyCode::Enter),
    ];
    run_events(&mut terminal, app, events);
    Ok(())
}

#[test]
fn test_event_loop_ctrl_backslash_root() -> io::Result<()> {
    let temp = TempDir::new()?;
    let app = setup_app(&temp)?;
    let mut terminal = setup_terminal()?;
    let events = vec![
        key_mod(KeyCode::Char('\\'), KeyModifiers::CONTROL),
        key(KeyCode::F(10)),
        // F10 now asks before quitting; Enter answers Yes.
        key(KeyCode::Enter),
    ];
    run_events(&mut terminal, app, events);
    Ok(())
}

// lib.rs:328,352 — Down key via event loop (coverage mode debounce)
#[test]
fn test_event_loop_down_up_navigation() -> io::Result<()> {
    let temp = TempDir::new()?;
    fs::write(temp.path().join("a.txt"), "a")?;
    fs::write(temp.path().join("b.txt"), "b")?;
    let app = setup_app(&temp)?;
    let mut terminal = setup_terminal()?;
    let events = vec![
        key(KeyCode::Down),
        key(KeyCode::Down),
        key(KeyCode::Up),
        key(KeyCode::F(10)),
        // F10 now asks before quitting; Enter answers Yes.
        key(KeyCode::Enter),
    ];
    run_events(&mut terminal, app, events);
    Ok(())
}

// lib.rs:370 — End key with entries (specific line coverage)
#[test]
fn test_event_loop_end_with_entries() -> io::Result<()> {
    let temp = TempDir::new()?;
    fs::write(temp.path().join("a.txt"), "a")?;
    fs::write(temp.path().join("b.txt"), "b")?;
    fs::write(temp.path().join("c.txt"), "c")?;
    let app = setup_app(&temp)?;
    let mut terminal = setup_terminal()?;
    let events = vec![key(KeyCode::End), key(KeyCode::F(10)), key(KeyCode::Enter)];
    run_events(&mut terminal, app, events);
    Ok(())
}

// Event loop — Left/Right arrow keys
#[test]
fn test_event_loop_left_right_arrows() -> io::Result<()> {
    let temp = TempDir::new()?;
    let app = setup_app(&temp)?;
    let mut terminal = setup_terminal()?;
    let events = vec![
        key(KeyCode::Right),
        key(KeyCode::Left),
        key(KeyCode::F(10)),
        key(KeyCode::Enter),
    ];
    run_events(&mut terminal, app, events);
    Ok(())
}

// Event loop — Enter navigates into directory
#[test]
fn test_event_loop_enter_directory() -> io::Result<()> {
    let temp = TempDir::new()?;
    let sub = temp.path().join("subdir");
    fs::create_dir(&sub)?;
    let app = setup_app(&temp)?;
    let mut terminal = setup_terminal()?;
    // Navigate to "subdir" then quit
    let events = vec![
        key(KeyCode::Down), // move to "subdir"
        key(KeyCode::Enter),
        key(KeyCode::F(10)),
        // F10 now asks before quitting; Enter answers Yes.
        key(KeyCode::Enter),
    ];
    run_events(&mut terminal, app, events);
    Ok(())
}
