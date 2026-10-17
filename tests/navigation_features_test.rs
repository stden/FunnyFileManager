//! Purpose: Exercises navigation features through the application event loop.
//!
use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};
use funny_file_manager::models::{App, PanelSide};
use funny_file_manager::run_app_with_event_reader;
use funny_file_manager::WritableTestBackend;
use ratatui::Terminal;
use std::fs;
use std::io;
use tempfile::TempDir;

const fn key_event(code: KeyCode) -> KeyEvent {
    KeyEvent::new(code, KeyModifiers::empty())
}

const fn key_event_with_mods(code: KeyCode, modifiers: KeyModifiers) -> KeyEvent {
    KeyEvent::new(code, modifiers)
}

#[test]
fn test_quick_search_navigation() -> io::Result<()> {
    let temp_dir = TempDir::new()?;
    fs::write(temp_dir.path().join("alpha.txt"), b"a")?;
    fs::write(temp_dir.path().join("beta.txt"), b"b")?;
    fs::write(temp_dir.path().join("gamma.txt"), b"g")?;

    let mut app = App::new()?;
    app.left_panel.path = temp_dir.path().to_path_buf();
    app.left_panel.load_entries()?;
    app.active_panel = PanelSide::Left;

    let backend = WritableTestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend)?;

    // Sequence: Alt+G (should jump to gamma), then Alt+A (should jump to alpha)
    let mut events = vec![
        Event::Key(key_event_with_mods(KeyCode::Char('g'), KeyModifiers::ALT)),
        Event::Key(key_event(KeyCode::F(10))),
        // F10 now asks before quitting; Enter answers Yes.
        Event::Key(key_event(KeyCode::Enter)),
    ]
    .into_iter();

    run_app_with_event_reader(&mut terminal, app.clone(), || {
        events
            .next()
            .ok_or_else(|| io::Error::new(io::ErrorKind::UnexpectedEof, "No more events"))
    })?;

    // We can't easily check the final state because run_app_with_event_reader takes ownership
    // and returns after 'q'.
    // But we can test the internal logic within App if we don't use run_app_with_event_reader.
    Ok(())
}

#[test]
fn test_ctrl_backslash_root_navigation() -> io::Result<()> {
    let temp_dir = TempDir::new()?;
    let mut app = App::new()?;
    app.left_panel.path = temp_dir.path().to_path_buf();

    let backend = WritableTestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend)?;

    let mut events = vec![
        Event::Key(key_event_with_mods(
            KeyCode::Char('\\'),
            KeyModifiers::CONTROL,
        )),
        Event::Key(key_event(KeyCode::F(10))),
        // F10 now asks before quitting; Enter answers Yes.
        Event::Key(key_event(KeyCode::Enter)),
    ]
    .into_iter();

    run_app_with_event_reader(&mut terminal, app, || {
        events
            .next()
            .ok_or_else(|| io::Error::new(io::ErrorKind::UnexpectedEof, "No more events"))
    })?;

    Ok(())
}
