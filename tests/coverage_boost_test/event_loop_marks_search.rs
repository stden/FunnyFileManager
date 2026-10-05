//! Purpose: Marking, quick search and the command line through the event loop.

use super::*;

#[test]
fn test_event_loop_insert_mark() -> io::Result<()> {
    let temp = TempDir::new()?;
    fs::write(temp.path().join("mark_me.txt"), "m")?;
    let mut app = setup_app(&temp)?;
    let idx = app
        .left_panel
        .entries
        .iter()
        .position(|e| e.name == "mark_me.txt")
        .unwrap();
    app.left_panel.select(Some(idx));
    let mut terminal = setup_terminal()?;
    let events = vec![
        key(KeyCode::Insert),
        key(KeyCode::F(10)),
        key(KeyCode::Enter),
    ];
    run_events(&mut terminal, app, events);
    Ok(())
}

#[test]
fn test_event_loop_space_mark() -> io::Result<()> {
    let temp = TempDir::new()?;
    fs::write(temp.path().join("space.txt"), "s")?;
    let mut app = setup_app(&temp)?;
    let idx = app
        .left_panel
        .entries
        .iter()
        .position(|e| e.name == "space.txt")
        .unwrap();
    app.left_panel.select(Some(idx));
    let mut terminal = setup_terminal()?;
    let events = vec![
        key(KeyCode::Char(' ')),
        key(KeyCode::F(10)),
        key(KeyCode::Enter),
    ];
    run_events(&mut terminal, app, events);
    Ok(())
}

// lib.rs:459 — Backspace in quick search
#[test]
fn test_event_loop_alt_letter_then_backspace() -> io::Result<()> {
    let temp = TempDir::new()?;
    fs::write(temp.path().join("alpha.txt"), "a")?;
    let app = setup_app(&temp)?;
    let mut terminal = setup_terminal()?;
    let events = vec![
        key_mod(KeyCode::Char('a'), KeyModifiers::ALT), // start quick search
        key(KeyCode::Char('l')),                        // extend search
        key(KeyCode::Backspace),                        // backspace in search
        key(KeyCode::Backspace),                        // backspace clears search
        key(KeyCode::F(10)),
        // F10 now asks before quitting; Enter answers Yes.
        key(KeyCode::Enter),
    ];
    run_events(&mut terminal, app, events);
    Ok(())
}

// lib.rs — Esc clears quick search
#[test]
fn test_event_loop_esc_clears_quick_search() -> io::Result<()> {
    let temp = TempDir::new()?;
    fs::write(temp.path().join("test.txt"), "t")?;
    let app = setup_app(&temp)?;
    let mut terminal = setup_terminal()?;
    let events = vec![
        key_mod(KeyCode::Char('t'), KeyModifiers::ALT),
        key(KeyCode::Esc),
        key(KeyCode::F(10)),
        // F10 now asks before quitting; Enter answers Yes.
        key(KeyCode::Enter),
    ];
    run_events(&mut terminal, app, events);
    Ok(())
}

// Event loop — Colon opens command line
#[test]
fn test_event_loop_colon_command_line() -> io::Result<()> {
    let temp = TempDir::new()?;
    let app = setup_app(&temp)?;
    let mut terminal = setup_terminal()?;
    let events = vec![
        key(KeyCode::Char(':')),
        key(KeyCode::Esc),
        key(KeyCode::F(10)),
        // F10 now asks before quitting; Enter answers Yes.
        key(KeyCode::Enter),
    ];
    run_events(&mut terminal, app, events);
    Ok(())
}

// Event loop — Slash opens search
#[test]
fn test_event_loop_slash_search() -> io::Result<()> {
    let temp = TempDir::new()?;
    let app = setup_app(&temp)?;
    let mut terminal = setup_terminal()?;
    let events = vec![
        key(KeyCode::Char('/')),
        key(KeyCode::Esc),
        key(KeyCode::F(10)),
        // F10 now asks before quitting; Enter answers Yes.
        key(KeyCode::Enter),
    ];
    run_events(&mut terminal, app, events);
    Ok(())
}
