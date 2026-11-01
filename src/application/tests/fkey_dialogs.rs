//! Unit tests: fkey dialogs.

use super::*;

use crate::models::{Panel, PanelSide};

use crossterm::event::{KeyCode, KeyModifiers};

use std::fs;

use std::path::PathBuf;

use tempfile::TempDir;

#[cfg(test)]
mod tests_file_operation_keys;
#[cfg(test)]
mod tests_panel_toggle_keys;
#[cfg(test)]
mod tests_user_menu_and_bookmarks;

// --- Shift+F4 new file dialog ---

#[test]
fn test_shift_f4_new_file_dialog() -> io::Result<()> {
    let temp_dir = TempDir::new().expect("test setup");
    let mut app = App::new().expect("test setup");
    app.left_panel = Panel::new(temp_dir.path().to_path_buf()).expect("test setup");
    app.right_panel = Panel::new(temp_dir.path().to_path_buf()).expect("test setup");

    let backend = WritableTestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).expect("test setup");

    let mut events = vec![
        // Shift+F4 opens new file dialog
        Event::Key(key_event_with_mods(KeyCode::F(4), KeyModifiers::SHIFT)),
        // Type filename
        Event::Key(key_event(KeyCode::Char('t'))),
        Event::Key(key_event(KeyCode::Char('e'))),
        Event::Key(key_event(KeyCode::Char('s'))),
        Event::Key(key_event(KeyCode::Char('t'))),
        // Confirm
        Event::Key(key_event(KeyCode::Enter)),
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
    assert!(temp_dir.path().join("test").exists());
    Ok(())
}

#[test]
fn test_run_app_shift_f4_renders_new_file_overlay() -> io::Result<()> {
    let temp_dir = TempDir::new().expect("test setup");
    let mut app = App::new().expect("test setup");
    app.left_panel = Panel::new(temp_dir.path().to_path_buf()).expect("test setup");
    app.right_panel = Panel::new(temp_dir.path().to_path_buf()).expect("test setup");

    let backend = WritableTestBackend::new(120, 24);
    let mut terminal = Terminal::new(backend).expect("test setup");

    let mut events = vec![Event::Key(key_event_with_mods(
        KeyCode::F(4),
        KeyModifiers::SHIFT,
    ))]
    .into_iter();

    let err = run_app_with_event_reader(&mut terminal, app, || {
        events.next().ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::UnexpectedEof,
                "stop after new-file overlay opens",
            )
        })
    })
    .expect_err("event reader should stop after rendering new-file overlay");

    assert_eq!(err.kind(), io::ErrorKind::UnexpectedEof);
    let screen = buffer_text(&terminal);
    assert!(screen.contains("Create New File (Shift+F4)"), "{screen}");
    Ok(())
}

// --- Alt+F1/Alt+F2 drive selection ---

#[test]
fn test_alt_f1_f2_drive_selection() -> io::Result<()> {
    let temp_dir = TempDir::new().expect("test setup");
    let mut app = App::new().expect("test setup");
    app.left_panel = Panel::new(temp_dir.path().to_path_buf()).expect("test setup");
    app.right_panel = Panel::new(temp_dir.path().to_path_buf()).expect("test setup");

    let backend = WritableTestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).expect("test setup");

    let mut events = vec![
        // Alt+F1 opens drive selection for left panel
        Event::Key(key_event_with_mods(KeyCode::F(1), KeyModifiers::ALT)),
        // Esc to close
        Event::Key(key_event(KeyCode::Esc)),
        // Alt+F2 opens drive selection for right panel
        Event::Key(key_event_with_mods(KeyCode::F(2), KeyModifiers::ALT)),
        // Esc to close
        Event::Key(key_event(KeyCode::Esc)),
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
