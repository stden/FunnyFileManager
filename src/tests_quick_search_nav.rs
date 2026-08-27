//! Unit tests: quick search nav (split out to keep files under 500 lines).
#![allow(unused_imports)]

use super::tests::*;

use super::*;

use crate::models::{Panel, PanelSide};

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use std::env;

use std::fs;

use std::path::{Path, PathBuf};

use std::sync::{Mutex, OnceLock};

use tempfile::TempDir;

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
        // Navigate - should NOT clear quick search
        Event::Key(key_event(KeyCode::Down)),
        Event::Key(key_event(KeyCode::Up)),
        // Tab should clear quick search
        Event::Key(key_event(KeyCode::Tab)),
        // Quit
        Event::Key(key_event(KeyCode::F(10))),
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
        // Tab should clear quick search
        Event::Key(key_event(KeyCode::Tab)),
        // Quit
        Event::Key(key_event(KeyCode::F(10))),
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
// Shift+F4 -- new file dialog
// -----------------------------------------------------------------------

#[test]
fn test_run_app_shift_f4_new_file() -> io::Result<()> {
    let temp_dir = TempDir::new().expect("test setup");
    fs::write(temp_dir.path().join("file.txt"), b"data").expect("test setup");

    let mut app = App::new().expect("test setup");
    app.left_panel = Panel::new(temp_dir.path().to_path_buf()).expect("test setup");
    app.right_panel = Panel::new(temp_dir.path().to_path_buf()).expect("test setup");

    let backend = WritableTestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).expect("test setup");

    let mut events = vec![
        // Shift+F4 opens the new-file input dialog
        Event::Key(key_event_with_mods(KeyCode::F(4), KeyModifiers::SHIFT)),
        // Esc inside the overlay to close it (handled by process_popup_input)
        Event::Key(key_event(KeyCode::Esc)),
        // Quit
        Event::Key(key_event(KeyCode::F(10))),
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
// Alt+F1 / Alt+F2 -- drive selection overlays
// -----------------------------------------------------------------------

#[test]
fn test_run_app_alt_f1_drive_selection() -> io::Result<()> {
    let temp_dir = TempDir::new().expect("test setup");

    let mut app = App::new().expect("test setup");
    app.left_panel = Panel::new(temp_dir.path().to_path_buf()).expect("test setup");
    app.right_panel = Panel::new(temp_dir.path().to_path_buf()).expect("test setup");

    let backend = WritableTestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).expect("test setup");

    let mut events = vec![
        // Alt+F1 opens drive selection for left panel
        Event::Key(key_event_with_mods(KeyCode::F(1), KeyModifiers::ALT)),
        // Esc closes the drive overlay (via process_popup_input)
        Event::Key(key_event(KeyCode::Esc)),
        // Quit
        Event::Key(key_event(KeyCode::F(10))),
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
fn test_run_app_alt_f2_drive_selection() -> io::Result<()> {
    let temp_dir = TempDir::new().expect("test setup");

    let mut app = App::new().expect("test setup");
    app.left_panel = Panel::new(temp_dir.path().to_path_buf()).expect("test setup");
    app.right_panel = Panel::new(temp_dir.path().to_path_buf()).expect("test setup");

    let backend = WritableTestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).expect("test setup");

    let mut events = vec![
        // Alt+F2 opens drive selection for right panel
        Event::Key(key_event_with_mods(KeyCode::F(2), KeyModifiers::ALT)),
        // Esc closes the drive overlay
        Event::Key(key_event(KeyCode::Esc)),
        // Quit
        Event::Key(key_event(KeyCode::F(10))),
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
fn test_run_app_alt_f1_renders_drive_overlay() -> io::Result<()> {
    let temp_dir = TempDir::new().expect("test setup");
    let mut app = App::new().expect("test setup");
    app.left_panel = Panel::new(temp_dir.path().to_path_buf()).expect("test setup");
    app.right_panel = Panel::new(temp_dir.path().to_path_buf()).expect("test setup");

    let backend = WritableTestBackend::new(120, 24);
    let mut terminal = Terminal::new(backend).expect("test setup");

    let mut events = vec![Event::Key(key_event_with_mods(
        KeyCode::F(1),
        KeyModifiers::ALT,
    ))]
    .into_iter();

    let err = run_app_with_event_reader(&mut terminal, app, || {
        events
            .next()
            .ok_or_else(|| io::Error::new(io::ErrorKind::UnexpectedEof, "stop after overlay opens"))
    })
    .expect_err("event reader should stop after rendering overlay");

    assert_eq!(err.kind(), io::ErrorKind::UnexpectedEof);
    let screen = buffer_text(&terminal);
    assert!(screen.contains("Select Drive (Left Panel)"), "{screen}");
    Ok(())
}

#[test]
fn test_run_app_alt_f2_renders_drive_overlay() -> io::Result<()> {
    let temp_dir = TempDir::new().expect("test setup");
    let mut app = App::new().expect("test setup");
    app.left_panel = Panel::new(temp_dir.path().to_path_buf()).expect("test setup");
    app.right_panel = Panel::new(temp_dir.path().to_path_buf()).expect("test setup");

    let backend = WritableTestBackend::new(120, 24);
    let mut terminal = Terminal::new(backend).expect("test setup");

    let mut events = vec![Event::Key(key_event_with_mods(
        KeyCode::F(2),
        KeyModifiers::ALT,
    ))]
    .into_iter();

    let err = run_app_with_event_reader(&mut terminal, app, || {
        events
            .next()
            .ok_or_else(|| io::Error::new(io::ErrorKind::UnexpectedEof, "stop after overlay opens"))
    })
    .expect_err("event reader should stop after rendering overlay");

    assert_eq!(err.kind(), io::ErrorKind::UnexpectedEof);
    let screen = buffer_text(&terminal);
    assert!(screen.contains("Select Drive (Right Panel)"), "{screen}");
    Ok(())
}

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
fn test_run_app_ctrl_o_renders_console_hint() -> io::Result<()> {
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
    assert!(screen.contains("Ctrl+O returns"), "{screen}");
    // Nothing has been run yet, so the transcript says so rather than showing
    // an unexplained black screen.
    assert!(screen.contains("Console is empty"), "{screen}");
    Ok(())
}
