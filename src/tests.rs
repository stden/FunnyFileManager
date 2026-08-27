//! Unit tests and shared test helpers for `lib`.
#![allow(unused_imports)]

use super::*;

use crate::models::{Panel, PanelSide};

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use std::fs;

use std::path::PathBuf;

use tempfile::TempDir;

pub fn key_event(code: KeyCode) -> KeyEvent {
    KeyEvent::new(code, KeyModifiers::empty())
}

pub fn key_event_with_mods(code: KeyCode, modifiers: KeyModifiers) -> KeyEvent {
    KeyEvent::new(code, modifiers)
}

pub fn buffer_text(terminal: &Terminal<WritableTestBackend>) -> String {
    let buf = terminal.backend().inner.buffer();
    let mut text = String::new();

    for y in 0..buf.area.height {
        for x in 0..buf.area.width {
            text.push_str(buf[(x, y)].symbol());
        }
        text.push('\n');
    }

    text
}

#[cfg(feature = "test-helpers")]
pub fn select_entry(panel: &mut Panel, name: &str) {
    if let Some(index) = panel.entries.iter().position(|entry| entry.name == name) {
        panel.select(Some(index));
    }
}

#[test]
fn dispatch_key_keeps_ctrl_u_behind_command_line_overlay() -> io::Result<()> {
    let left_dir = TempDir::new().expect("left temp dir");
    let right_dir = TempDir::new().expect("right temp dir");
    let mut app = App::new().expect("test app");
    app.left_panel = Panel::new(left_dir.path().to_path_buf()).expect("left panel");
    app.right_panel = Panel::new(right_dir.path().to_path_buf()).expect("right panel");
    app.set_overlay(Overlay::CommandLine("pwd".into()));
    let left_path = app.left_panel.path.clone();
    let right_path = app.right_panel.path.clone();

    let backend = WritableTestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).expect("test terminal");
    let mut last_nav_time = Instant::now();

    dispatch_key(
        &mut terminal,
        &mut app,
        KeyEvent::new(KeyCode::Char('u'), KeyModifiers::CONTROL),
        &mut last_nav_time,
        Duration::ZERO,
    );

    assert!(app.lifecycle.is_running());
    // Inside the prompt Ctrl+U kills to the start of the line; what it must
    // never do is reach `Action::SwapPanels` behind the overlay.
    assert!(matches!(
        &app.overlay,
        Overlay::CommandLine(state) if state.text().is_empty()
    ));
    assert_eq!(app.left_panel.path, left_path);
    assert_eq!(app.right_panel.path, right_path);
    Ok(())
}

#[test]
#[cfg(feature = "test-helpers")]
fn test_run_app_with_event_sequence() -> io::Result<()> {
    let temp_dir = TempDir::new().expect("test setup");
    let left_dir = temp_dir.path().join("left");
    let right_dir = temp_dir.path().join("right");
    fs::create_dir(&left_dir).expect("test setup");
    fs::create_dir(&right_dir).expect("test setup");

    fs::write(left_dir.join("left.txt"), b"left").expect("test setup");
    fs::write(right_dir.join("right.txt"), b"right").expect("test setup");

    let mut app = App::new().expect("test setup");
    app.left_panel = Panel::new(left_dir).expect("test setup");
    app.right_panel = Panel::new(right_dir).expect("test setup");
    app.active_panel = PanelSide::Left;
    select_entry(&mut app.left_panel, "left.txt");
    select_entry(&mut app.right_panel, "right.txt");
    app.set_overlay(Overlay::Help);

    let backend = WritableTestBackend::new(240, 24);
    let mut terminal = Terminal::new(backend).expect("test setup");

    let mut events = vec![
        Event::Key(key_event(KeyCode::Esc)),
        Event::Key(key_event(KeyCode::F(1))),
        Event::Key(key_event(KeyCode::Esc)),
        Event::Key(key_event(KeyCode::Down)),
        Event::Key(key_event(KeyCode::Up)),
        Event::Key(key_event(KeyCode::Enter)),
        Event::Key(key_event(KeyCode::Tab)),
        Event::Key(key_event(KeyCode::Left)),
        Event::Key(key_event(KeyCode::Right)),
        Event::Key(key_event(KeyCode::F(3))),
        Event::Key(key_event(KeyCode::Esc)),
        Event::Key(key_event(KeyCode::F(4))),
        Event::Key(key_event(KeyCode::Esc)),
        Event::Key(key_event(KeyCode::F(5))),
        Event::Key(key_event(KeyCode::F(6))),
        Event::Key(key_event(KeyCode::F(7))),
        Event::Key(key_event(KeyCode::F(8))),
        // A printable key now starts a command; Esc abandons the prompt so
        // F10 reaches the main view rather than being swallowed by it.
        Event::Key(key_event(KeyCode::Char('x'))),
        Event::Key(key_event(KeyCode::Esc)),
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
fn test_run_app_ctrl_c_quit() -> io::Result<()> {
    let temp_dir = TempDir::new().expect("test setup");
    fs::write(temp_dir.path().join("file.txt"), b"data").expect("test setup");

    let mut app = App::new().expect("test setup");
    app.left_panel = Panel::new(temp_dir.path().to_path_buf()).expect("test setup");
    app.right_panel = Panel::new(temp_dir.path().to_path_buf()).expect("test setup");

    let backend = WritableTestBackend::new(240, 24);
    let mut terminal = Terminal::new(backend).expect("test setup");

    let mut events = vec![Event::Key(key_event_with_mods(
        KeyCode::Char('c'),
        KeyModifiers::CONTROL,
    ))]
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
fn test_run_app_command_line_quit() -> io::Result<()> {
    let temp_dir = TempDir::new().expect("test setup");
    let mut app = App::new().expect("test setup");
    app.left_panel = Panel::new(temp_dir.path().to_path_buf()).expect("test setup");
    app.right_panel = Panel::new(temp_dir.path().to_path_buf()).expect("test setup");
    app.set_overlay(Overlay::CommandLine("quit".into()));

    let backend = WritableTestBackend::new(240, 24);
    let mut terminal = Terminal::new(backend).expect("test setup");

    let mut events = vec![Event::Key(key_event(KeyCode::Enter))].into_iter();
    let result = run_app_with_event_reader(&mut terminal, app, || {
        events
            .next()
            .ok_or_else(|| io::Error::new(io::ErrorKind::UnexpectedEof, "No more events"))
    });

    assert!(result.is_ok());
    Ok(())
}

#[test]
fn test_run_app_should_quit_after_key() -> io::Result<()> {
    let temp_dir = TempDir::new().expect("test setup");
    let mut app = App::new().expect("test setup");
    app.left_panel = Panel::new(temp_dir.path().to_path_buf()).expect("test setup");
    app.right_panel = Panel::new(temp_dir.path().to_path_buf()).expect("test setup");

    let backend = WritableTestBackend::new(240, 24);
    let mut terminal = Terminal::new(backend).expect("test setup");

    let mut events = vec![Event::Key(key_event(KeyCode::F(10)))].into_iter();
    let result = run_app_with_event_reader(&mut terminal, app, || {
        events
            .next()
            .ok_or_else(|| io::Error::new(io::ErrorKind::UnexpectedEof, "No more events"))
    });

    assert!(result.is_ok());
    Ok(())
}

#[test]
fn test_run_app_debounce_paths() -> io::Result<()> {
    let temp_dir = TempDir::new().expect("test setup");
    let mut app = App::new().expect("test setup");
    app.left_panel = Panel::new(temp_dir.path().to_path_buf()).expect("test setup");
    app.right_panel = Panel::new(temp_dir.path().to_path_buf()).expect("test setup");

    let backend = WritableTestBackend::new(240, 24);
    let mut terminal = Terminal::new(backend).expect("test setup");

    let mut events = vec![
        Event::Key(key_event(KeyCode::Down)),
        Event::Key(key_event(KeyCode::Up)),
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
fn test_run_app_navigation_paths() -> io::Result<()> {
    let temp_dir = TempDir::new().expect("test setup");
    let mut app = App::new().expect("test setup");
    app.left_panel = Panel::new(temp_dir.path().to_path_buf()).expect("test setup");
    app.right_panel = Panel::new(temp_dir.path().to_path_buf()).expect("test setup");

    let backend = WritableTestBackend::new(240, 24);
    let mut terminal = Terminal::new(backend).expect("test setup");

    let mut events = vec![
        Event::Key(key_event(KeyCode::Down)),
        Event::Key(key_event(KeyCode::Up)),
        Event::Key(key_event(KeyCode::F(10))),
    ]
    .into_iter();
    let result = run_app_with_event_reader(&mut terminal, app, || {
        #[cfg(coverage)]
        std::thread::sleep(Duration::from_millis(300));
        events
            .next()
            .ok_or_else(|| io::Error::new(io::ErrorKind::UnexpectedEof, "No more events"))
    });

    assert!(result.is_ok());
    Ok(())
}

#[test]
fn test_run_with_logs_records_error_at_explicit_paths() -> io::Result<()> {
    let temp = TempDir::new()?;
    let actions = temp.path().join("actions.log");
    let errors = temp.path().join("errors.log");

    let result = run_with_logs(|| Err(io::Error::other("boom").into()), &actions, &errors);

    assert!(result.is_err());
    assert!(fs::read_to_string(actions)?.contains("Started"));
    assert!(fs::read_to_string(errors)?.contains("Main error: boom"));
    Ok(())
}

#[test]
fn test_run_with_logs_records_success_at_explicit_paths() -> io::Result<()> {
    let temp = TempDir::new()?;
    let actions = temp.path().join("actions.log");
    let errors = temp.path().join("errors.log");

    run_with_logs(|| Ok(()), &actions, &errors).expect("successful runner");

    let contents = fs::read_to_string(actions)?;
    assert!(contents.contains("Started"));
    assert!(contents.contains("Ended"));
    assert!(!errors.exists());
    Ok(())
}

#[test]
fn test_log_writers_ignore_unwritable_explicit_paths() -> io::Result<()> {
    let temp = TempDir::new()?;
    let actions = temp.path().join("actions");
    let errors = temp.path().join("errors");
    fs::create_dir(&actions)?;
    fs::create_dir(&errors)?;

    log_action_to(&actions, "action");
    log_error_to(&errors, "error");

    assert!(actions.is_dir());
    assert!(errors.is_dir());
    Ok(())
}

#[test]
fn test_writable_test_backend_write() -> io::Result<()> {
    let mut backend = WritableTestBackend::new(80, 24);
    let data = b"test data";
    let written = backend.write(data).expect("test setup");
    assert_eq!(written, data.len());
    Ok(())
}

#[test]
fn test_writable_test_backend_flush() -> io::Result<()> {
    let mut backend = WritableTestBackend::new(80, 24);
    Write::flush(&mut backend).expect("test setup");
    Backend::flush(&mut backend).expect("test setup");
    Ok(())
}

#[test]
fn test_writable_test_backend_cursor_operations() -> io::Result<()> {
    let mut backend = WritableTestBackend::new(80, 24);

    let pos = backend.get_cursor_position().expect("test setup");
    assert_eq!((pos.x, pos.y), (0, 0));

    backend
        .set_cursor_position(ratatui::layout::Position::new(10, 5))
        .expect("test setup");
    let pos = backend.get_cursor_position().expect("test setup");
    assert_eq!((pos.x, pos.y), (10, 5));

    Ok(())
}

#[test]
fn test_writable_test_backend_clear() -> io::Result<()> {
    let mut backend = WritableTestBackend::new(80, 24);
    backend.clear().expect("test setup");
    Ok(())
}

#[test]
fn test_writable_test_backend_window_size() -> io::Result<()> {
    let mut backend = WritableTestBackend::new(80, 24);
    let window_size = backend.window_size().expect("test setup");

    assert_eq!(window_size.columns_rows.width, 80);
    assert_eq!(window_size.columns_rows.height, 24);
    assert_eq!(window_size.pixels.width, 80 * 8);
    assert_eq!(window_size.pixels.height, 24 * 16);

    Ok(())
}

#[test]
fn test_run_app_full_coverage() -> io::Result<()> {
    let temp_dir = TempDir::new().expect("test setup");
    let mut app = App::new().expect("test setup");
    app.left_panel = Panel::new(temp_dir.path().to_path_buf()).expect("test setup");
    app.right_panel = Panel::new(temp_dir.path().to_path_buf()).expect("test setup");

    let backend = WritableTestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).expect("test setup");

    let mut events = vec![Event::Key(key_event(KeyCode::F(10)))].into_iter();

    run_app_with_event_reader(&mut terminal, app, || {
        events
            .next()
            .ok_or_else(|| io::Error::new(io::ErrorKind::UnexpectedEof, "No more events"))
    })
    .expect("test setup");

    Ok(())
}
