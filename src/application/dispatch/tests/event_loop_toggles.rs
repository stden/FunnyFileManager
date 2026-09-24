//! Unit tests: event loop toggles (split out to keep files under 500 lines).

use super::*;

use crate::models::Panel;

use crossterm::event::{KeyCode, KeyModifiers};

use std::fs;

use tempfile::TempDir;

// -----------------------------------------------------------------------
// Ctrl+U -- swap panels
// -----------------------------------------------------------------------

#[test]
fn test_run_app_ctrl_u_swap_panels() -> io::Result<()> {
    let temp_dir = TempDir::new().expect("test setup");
    let left_dir = temp_dir.path().join("left");
    let right_dir = temp_dir.path().join("right");
    fs::create_dir(&left_dir).expect("test setup");
    fs::create_dir(&right_dir).expect("test setup");

    let mut app = App::new().expect("test setup");
    app.left_panel = Panel::new(left_dir).expect("test setup");
    app.right_panel = Panel::new(right_dir).expect("test setup");

    let backend = WritableTestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).expect("test setup");

    let mut events = vec![
        // Ctrl+U swaps panel paths
        Event::Key(key_event_with_mods(
            KeyCode::Char('u'),
            KeyModifiers::CONTROL,
        )),
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
fn test_run_app_ctrl_u_renders_status_message() -> io::Result<()> {
    let temp_dir = TempDir::new().expect("test setup");
    let left_dir = temp_dir.path().join("left");
    let right_dir = temp_dir.path().join("right");
    fs::create_dir(&left_dir).expect("test setup");
    fs::create_dir(&right_dir).expect("test setup");

    let mut app = App::new().expect("test setup");
    app.left_panel = Panel::new(left_dir).expect("test setup");
    app.right_panel = Panel::new(right_dir).expect("test setup");

    let backend = WritableTestBackend::new(240, 24);
    let mut terminal = Terminal::new(backend).expect("test setup");

    let mut events = vec![Event::Key(key_event_with_mods(
        KeyCode::Char('u'),
        KeyModifiers::CONTROL,
    ))]
    .into_iter();

    let err = run_app_with_event_reader(&mut terminal, app, || {
        events.next().ok_or_else(|| {
            io::Error::new(io::ErrorKind::UnexpectedEof, "stop after status renders")
        })
    })
    .expect_err("event reader should stop after rendering status");

    assert_eq!(err.kind(), io::ErrorKind::UnexpectedEof);
    let screen = buffer_text(&terminal);
    assert!(screen.contains("Panels swapped"), "{screen}");
    Ok(())
}

// -----------------------------------------------------------------------
// F1 toggles show_help state
// -----------------------------------------------------------------------

#[test]
fn test_f1_key_toggles_show_help_on() -> io::Result<()> {
    // Verify that F1 exercises the show_help toggle code path in the event
    // loop.  Since run_app_with_event_reader consumes the app we cannot
    // inspect state afterwards, so we just check the run completes without
    // error.
    let temp_dir = TempDir::new().expect("test setup");
    let mut app = App::new().expect("test setup");
    app.left_panel = Panel::new(temp_dir.path().to_path_buf()).expect("test setup");
    app.right_panel = Panel::new(temp_dir.path().to_path_buf()).expect("test setup");

    let backend = WritableTestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).expect("test setup");

    // F1 toggles help ON; F1 again closes it; then F10 quits.
    let mut events = vec![
        Event::Key(key_event(KeyCode::F(1))),
        Event::Key(key_event(KeyCode::F(1))),
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
fn test_run_app_f1_renders_help_overlay() -> io::Result<()> {
    let temp_dir = TempDir::new().expect("test setup");
    let mut app = App::new().expect("test setup");
    app.left_panel = Panel::new(temp_dir.path().to_path_buf()).expect("test setup");
    app.right_panel = Panel::new(temp_dir.path().to_path_buf()).expect("test setup");

    let backend = WritableTestBackend::new(160, 30);
    let mut terminal = Terminal::new(backend).expect("test setup");

    let mut events = vec![Event::Key(key_event(KeyCode::F(1)))].into_iter();

    let err = run_app_with_event_reader(&mut terminal, app, || {
        events.next().ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::UnexpectedEof,
                "stop after help overlay renders",
            )
        })
    })
    .expect_err("event reader should stop after rendering help overlay");

    assert_eq!(err.kind(), io::ErrorKind::UnexpectedEof);
    let screen = buffer_text(&terminal);
    assert!(screen.contains("Far Manager style navigation:"), "{screen}");
    assert!(screen.contains("F1: Toggle this help window"), "{screen}");
    Ok(())
}

#[test]
fn test_apply_action_toggle_help_and_console() -> io::Result<()> {
    let mut app = App::new()?;
    let backend = WritableTestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend)?;

    apply_action(&mut terminal, &mut app, Action::ToggleHelp);
    assert!(matches!(app.overlay, Overlay::Help));

    apply_action(&mut terminal, &mut app, Action::ToggleHelp);
    assert!(matches!(app.overlay, Overlay::None));

    apply_action(&mut terminal, &mut app, Action::ToggleConsole);
    assert_eq!(app.view, ViewMode::Console);

    apply_action(&mut terminal, &mut app, Action::ToggleConsole);
    assert!(matches!(app.overlay, Overlay::None));

    Ok(())
}

mod ignored_events;
mod refresh_keys;
mod viewer_keys;
