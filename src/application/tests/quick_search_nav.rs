//! Unit tests: quick search nav (split out to keep files under 500 lines).
#![allow(unused_imports)]

use super::*;

use crate::models::Panel;

use crossterm::event::{KeyCode, KeyModifiers};

use std::fs;

use tempfile::TempDir;

#[cfg(test)]
mod console_toggle;
#[cfg(test)]
mod overlay_hotkeys;

#[test]
fn test_quick_search_cleared_on_navigation() -> io::Result<()> {
    let temp_dir = TempDir::new().expect("test setup");
    fs::write(temp_dir.path().join("file.txt"), b"data").expect("test setup");

    let mut app = App::new().expect("test setup");
    app.left_panel = Panel::new(temp_dir.path().to_path_buf()).expect("test setup");
    app.right_panel = Panel::new(temp_dir.path().to_path_buf()).expect("test setup");

    let backend = WritableTestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).expect("test setup");

    let mut events = vec![
        // Start quick search
        Event::Key(key_event_with_mods(KeyCode::Char('f'), KeyModifiers::ALT)),
        // Navigation ends the quick search, as in Far (see quick_search_ends)
        Event::Key(key_event(KeyCode::Down)),
        Event::Key(key_event(KeyCode::Up)),
        // Tab ends the quick search too
        Event::Key(key_event(KeyCode::Tab)),
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
fn test_quick_search_cleared_on_tab() -> io::Result<()> {
    let temp_dir = TempDir::new().expect("test setup");
    fs::write(temp_dir.path().join("file.txt"), b"data").expect("test setup");

    let mut app = App::new().expect("test setup");
    app.left_panel.path = temp_dir.path().to_path_buf();
    app.right_panel.path = temp_dir.path().to_path_buf();

    let backend = WritableTestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).expect("test setup");

    let mut events = vec![
        // Start quick search
        Event::Key(key_event_with_mods(KeyCode::Char('f'), KeyModifiers::ALT)),
        // Tab ends the quick search too
        Event::Key(key_event(KeyCode::Tab)),
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

// --- Non-key events (Mouse, Resize) should be ignored ---

#[test]
fn test_non_key_events_ignored() -> io::Result<()> {
    let temp_dir = TempDir::new().expect("test setup");
    let mut app = App::new().expect("test setup");
    app.left_panel = Panel::new(temp_dir.path().to_path_buf()).expect("test setup");
    app.right_panel = Panel::new(temp_dir.path().to_path_buf()).expect("test setup");

    let backend = WritableTestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).expect("test setup");

    let mut events = vec![
        // Resize event - should be ignored
        Event::Resize(120, 40),
        // Another resize
        Event::Resize(80, 24),
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

// -----------------------------------------------------------------------
// Home and End key navigation
// -----------------------------------------------------------------------

#[test]
fn test_run_app_home_end_keys() -> io::Result<()> {
    let temp_dir = TempDir::new().expect("test setup");
    for i in 1..=5 {
        fs::write(temp_dir.path().join(format!("file{i}.txt")), b"data").expect("test setup");
    }

    let mut app = App::new().expect("test setup");
    app.left_panel = Panel::new(temp_dir.path().to_path_buf()).expect("test setup");
    app.right_panel = Panel::new(temp_dir.path().to_path_buf()).expect("test setup");

    let backend = WritableTestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).expect("test setup");

    let mut events = vec![
        // End - jump to last entry
        Event::Key(key_event(KeyCode::End)),
        // Home - jump to first entry
        Event::Key(key_event(KeyCode::Home)),
        // End again for good measure
        Event::Key(key_event(KeyCode::End)),
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
fn test_run_app_end_key_empty_panel() -> io::Result<()> {
    // End on an empty panel should not panic (entries.is_empty() branch).
    let temp_dir = TempDir::new().expect("test setup");

    let mut app = App::new().expect("test setup");
    app.left_panel = Panel::new(temp_dir.path().to_path_buf()).expect("test setup");
    app.right_panel = Panel::new(temp_dir.path().to_path_buf()).expect("test setup");
    // Make sure there are no entries
    app.left_panel.entries.clear();
    app.left_panel.select(None);

    let backend = WritableTestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).expect("test setup");

    let mut events = vec![
        Event::Key(key_event(KeyCode::End)),
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
