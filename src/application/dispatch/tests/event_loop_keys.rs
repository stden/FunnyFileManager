//! Unit tests: event loop keys (split out to keep files under 500 lines).

use super::*;

use crate::models::{Panel, PanelSide};

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use std::fs;

use tempfile::TempDir;

#[test]
fn test_all_key_handlers_coverage() -> io::Result<()> {
    let temp_dir = TempDir::new().expect("test setup");
    let subdir = temp_dir.path().join("subdir");
    fs::create_dir(&subdir).expect("test setup");
    fs::write(temp_dir.path().join("file1.txt"), b"content1").expect("test setup");
    fs::write(temp_dir.path().join("file2.txt"), b"content2").expect("test setup");

    let mut app = App::new().expect("test setup");
    app.left_panel = Panel::new(temp_dir.path().to_path_buf()).expect("test setup");
    app.right_panel = Panel::new(temp_dir.path().to_path_buf()).expect("test setup");

    let backend = WritableTestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).expect("test setup");

    let mut events = navigation_dialog_events();

    run_app_with_event_reader(&mut terminal, app, || {
        events
            .next()
            .ok_or_else(|| io::Error::new(io::ErrorKind::UnexpectedEof, "No more events"))
    })
    .expect("test setup");

    Ok(())
}

#[test]
fn test_overlay_continue_path() -> io::Result<()> {
    let temp_dir = TempDir::new().expect("test setup");
    fs::write(temp_dir.path().join("file.txt"), b"content").expect("test setup");

    let mut app = App::new().expect("test setup");
    app.left_panel = Panel::new(temp_dir.path().to_path_buf()).expect("test setup");
    app.right_panel = Panel::new(temp_dir.path().to_path_buf()).expect("test setup");
    // Start with help overlay active
    app.set_overlay(Overlay::Help);

    let backend = WritableTestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).expect("test setup");

    let mut events = vec![
        // This key is handled by overlay, triggering continue
        Event::Key(key_event(KeyCode::Esc)),
        // After overlay closes, this quits
        Event::Key(key_event(KeyCode::F(10))),
        // F10 now asks before quitting; Enter answers Yes.
        Event::Key(key_event(KeyCode::Enter)),
    ]
    .into_iter();

    run_app_with_event_reader(&mut terminal, app, || {
        events
            .next()
            .ok_or_else(|| io::Error::new(io::ErrorKind::UnexpectedEof, "No more events"))
    })
    .expect("test setup");

    Ok(())
}

#[test]
fn test_function_keys_f4_f5_f6_f8() -> io::Result<()> {
    let temp_dir = TempDir::new().expect("test setup");
    let file_path = temp_dir.path().join("file.txt");
    fs::write(&file_path, b"content").expect("test setup");

    let mut app = App::new().expect("test setup");
    app.left_panel = Panel::new(temp_dir.path().to_path_buf()).expect("test setup");
    app.right_panel = Panel::new(temp_dir.path().to_path_buf()).expect("test setup");

    let backend = WritableTestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).expect("test setup");

    let mut events = vec![
        // F4 edit (opens editor overlay, then close)
        Event::Key(key_event(KeyCode::F(4))),
        Event::Key(key_event(KeyCode::Esc)),
        // F5 copy (select file first). F5 opens a dialog now, so it needs an
        // answer before the next key is meant for the panel again.
        Event::Key(key_event(KeyCode::Down)),
        Event::Key(key_event(KeyCode::F(5))),
        Event::Key(key_event(KeyCode::Esc)),
        // F6 move (cancel via Esc since no confirmation needed)
        Event::Key(key_event(KeyCode::F(6))),
        // F8 delete (cancel)
        Event::Key(key_event(KeyCode::F(8))),
        Event::Key(key_event(KeyCode::Esc)),
        // Quit
        Event::Key(key_event(KeyCode::F(10))),
        // F10 now asks before quitting; Enter answers Yes.
        Event::Key(key_event(KeyCode::Enter)),
    ]
    .into_iter();

    run_app_with_event_reader(&mut terminal, app, || {
        events
            .next()
            .ok_or_else(|| io::Error::new(io::ErrorKind::UnexpectedEof, "No more events"))
    })
    .expect("test setup");

    Ok(())
}

#[test]
fn test_far_manager_hotkeys() -> io::Result<()> {
    let temp_dir = TempDir::new().expect("test setup");
    fs::create_dir(temp_dir.path().join("subdir")).expect("test setup");
    fs::write(temp_dir.path().join("file1.txt"), b"data").expect("test setup");
    fs::write(temp_dir.path().join("file2.txt"), b"data").expect("test setup");

    let mut app = App::new().expect("test setup");
    app.left_panel = Panel::new(temp_dir.path().to_path_buf()).expect("test setup");
    app.right_panel = Panel::new(temp_dir.path().to_path_buf()).expect("test setup");

    let backend = WritableTestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).expect("test setup");

    let mut events = vec![
        // Test Insert key for marking
        Event::Key(key_event(KeyCode::Insert)),
        // Test Space key for marking
        Event::Key(key_event(KeyCode::Char(' '))),
        // Test Ctrl+\ for root navigation
        Event::Key(key_event_with_mods(
            KeyCode::Char('\\'),
            KeyModifiers::CONTROL,
        )),
        // Exit
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

fn navigation_dialog_events() -> impl Iterator<Item = Event> {
    vec![
        // Navigation keys
        Event::Key(key_event(KeyCode::Down)),
        Event::Key(key_event(KeyCode::Up)),
        // Panel switching
        Event::Key(key_event(KeyCode::Tab)),
        Event::Key(key_event(KeyCode::Left)),
        Event::Key(key_event(KeyCode::Right)),
        // F1 toggle help
        Event::Key(key_event(KeyCode::F(1))),
        Event::Key(key_event(KeyCode::Esc)),
        // F3 view (then close)
        Event::Key(key_event(KeyCode::F(3))),
        Event::Key(key_event(KeyCode::Esc)),
        // F7 mkdir (then cancel)
        Event::Key(key_event(KeyCode::F(7))),
        Event::Key(key_event(KeyCode::Esc)),
        // Enter key on directory
        Event::Key(key_event(KeyCode::Down)),
        Event::Key(key_event(KeyCode::Enter)),
        // Backspace with no quick search
        Event::Key(key_event(KeyCode::Backspace)),
        // A printable key starts a command; Esc abandons the prompt so the
        // quit key below reaches the main view instead of the overlay.
        Event::Key(key_event(KeyCode::Char('x'))),
        Event::Key(key_event(KeyCode::Esc)),
        // Quit
        Event::Key(key_event(KeyCode::F(10))),
        // F10 now asks before quitting; Enter answers Yes.
        Event::Key(key_event(KeyCode::Enter)),
    ]
    .into_iter()
}

mod cursor_movement;
mod quick_search;
mod typed_commands;
