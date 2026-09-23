//! Unit tests and shared test helpers for `lib`.

use super::*;

use crate::models::Panel;
#[cfg(feature = "test-helpers")]
use crate::models::PanelSide;

use std::fs;

use tempfile::TempDir;

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

    dispatch_key(
        &mut terminal,
        &mut app,
        KeyEvent::new(KeyCode::Char('u'), KeyModifiers::CONTROL),
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

    let mut events = file_workflow_events();

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

    let mut events = vec![
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
fn test_run_app_full_coverage() -> io::Result<()> {
    let temp_dir = TempDir::new().expect("test setup");
    let mut app = App::new().expect("test setup");
    app.left_panel = Panel::new(temp_dir.path().to_path_buf()).expect("test setup");
    app.right_panel = Panel::new(temp_dir.path().to_path_buf()).expect("test setup");

    let backend = WritableTestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).expect("test setup");

    let mut events = vec![
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

#[cfg(feature = "test-helpers")]
fn file_workflow_events() -> impl Iterator<Item = Event> {
    vec![
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
        // F10 now asks before quitting; Enter answers Yes.
        Event::Key(key_event(KeyCode::Enter)),
    ]
    .into_iter()
}
