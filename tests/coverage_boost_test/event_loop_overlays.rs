//! Purpose: Keys that open overlays or end the session, through the event loop.

use super::*;

// ═══════════════════════════════════════════════════════════════════
// Event loop edge cases via run_app_with_event_reader
// ═══════════════════════════════════════════════════════════════════

#[test]
fn test_event_loop_ctrl_o_console_toggle() -> io::Result<()> {
    let temp = TempDir::new()?;
    let app = setup_app(&temp)?;
    let mut terminal = setup_terminal()?;
    let events = vec![
        key_mod(KeyCode::Char('o'), KeyModifiers::CONTROL),
        key_mod(KeyCode::Char('o'), KeyModifiers::CONTROL),
        key(KeyCode::F(10)),
        // F10 now asks before quitting; Enter answers Yes.
        key(KeyCode::Enter),
    ];
    run_events(&mut terminal, app, events);
    Ok(())
}

#[test]
fn test_event_loop_ctrl_u_swap() -> io::Result<()> {
    let temp = TempDir::new()?;
    let app = setup_app(&temp)?;
    let mut terminal = setup_terminal()?;
    let events = vec![
        key_mod(KeyCode::Char('u'), KeyModifiers::CONTROL),
        key(KeyCode::F(10)),
        // F10 now asks before quitting; Enter answers Yes.
        key(KeyCode::Enter),
    ];
    run_events(&mut terminal, app, events);
    Ok(())
}

#[test]
fn test_event_loop_f1_help() -> io::Result<()> {
    let temp = TempDir::new()?;
    let app = setup_app(&temp)?;
    let mut terminal = setup_terminal()?;
    let events = vec![
        key(KeyCode::F(1)),
        key(KeyCode::Esc),
        key(KeyCode::F(10)),
        key(KeyCode::Enter),
    ];
    run_events(&mut terminal, app, events);
    Ok(())
}

#[test]
fn test_event_loop_f10_quit() -> io::Result<()> {
    let temp = TempDir::new()?;
    let app = setup_app(&temp)?;
    let mut terminal = setup_terminal()?;
    let events = vec![key(KeyCode::F(10)), key(KeyCode::Enter)];
    run_events(&mut terminal, app, events);
    Ok(())
}

#[test]
fn test_event_loop_alt_f1_drives() -> io::Result<()> {
    let temp = TempDir::new()?;
    let app = setup_app(&temp)?;
    let mut terminal = setup_terminal()?;
    let events = vec![
        key_mod(KeyCode::F(1), KeyModifiers::ALT),
        key(KeyCode::Esc),
        key(KeyCode::F(10)),
        // F10 now asks before quitting; Enter answers Yes.
        key(KeyCode::Enter),
    ];
    run_events(&mut terminal, app, events);
    Ok(())
}

#[test]
fn test_event_loop_alt_f2_drives() -> io::Result<()> {
    let temp = TempDir::new()?;
    let app = setup_app(&temp)?;
    let mut terminal = setup_terminal()?;
    let events = vec![
        key_mod(KeyCode::F(2), KeyModifiers::ALT),
        key(KeyCode::Esc),
        key(KeyCode::F(10)),
        // F10 now asks before quitting; Enter answers Yes.
        key(KeyCode::Enter),
    ];
    run_events(&mut terminal, app, events);
    Ok(())
}

// Event loop — Ctrl+C quit
#[test]
fn test_event_loop_ctrl_c_quit() -> io::Result<()> {
    let temp = TempDir::new()?;
    let app = setup_app(&temp)?;
    let mut terminal = setup_terminal()?;
    let events = vec![key_mod(KeyCode::Char('c'), KeyModifiers::CONTROL)];
    run_events(&mut terminal, app, events);
    Ok(())
}

// Event loop — F3 view
#[test]
fn test_event_loop_f3_view() -> io::Result<()> {
    let temp = TempDir::new()?;
    fs::write(temp.path().join("viewable.txt"), "contents")?;
    let mut app = setup_app(&temp)?;
    let idx = app
        .left_panel
        .entries
        .iter()
        .position(|e| e.name == "viewable.txt")
        .unwrap();
    app.left_panel.select(Some(idx));
    let mut terminal = setup_terminal()?;
    let events = vec![
        key(KeyCode::F(3)),
        key(KeyCode::Esc),
        key(KeyCode::F(10)),
        key(KeyCode::Enter),
    ];
    run_events(&mut terminal, app, events);
    Ok(())
}

// Event loop — F8 delete with confirm
#[test]
fn test_event_loop_f8_delete_confirm_cancel() -> io::Result<()> {
    let temp = TempDir::new()?;
    fs::write(temp.path().join("target.txt"), "bye")?;
    let mut app = setup_app(&temp)?;
    let idx = app
        .left_panel
        .entries
        .iter()
        .position(|e| e.name == "target.txt")
        .unwrap();
    app.left_panel.select(Some(idx));
    let mut terminal = setup_terminal()?;
    let events = vec![
        key(KeyCode::F(8)),
        key(KeyCode::Char('n')), // cancel
        key(KeyCode::F(10)),
        // F10 now asks before quitting; Enter answers Yes.
        key(KeyCode::Enter),
    ];
    run_events(&mut terminal, app, events);
    assert!(temp.path().join("target.txt").exists()); // still there
    Ok(())
}
