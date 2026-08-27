//! Unit tests: event loop toggles (split out to keep files under 500 lines).
#![allow(unused_imports)]

use super::tests::*;
use crate::models::ViewMode;

use super::*;

use crate::models::{Panel, PanelSide};

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use std::env;

use std::fs;

use std::path::{Path, PathBuf};

use std::sync::{Mutex, OnceLock};

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
// KeyEventKind::Release -- ignored (continue branch)
// -----------------------------------------------------------------------

#[test]
fn test_run_app_release_event_ignored() -> io::Result<()> {
    let temp_dir = TempDir::new().expect("test setup");

    let mut app = App::new().expect("test setup");
    app.left_panel = Panel::new(temp_dir.path().to_path_buf()).expect("test setup");
    app.right_panel = Panel::new(temp_dir.path().to_path_buf()).expect("test setup");

    let backend = WritableTestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).expect("test setup");

    // Build a Release key event (should be silently ignored, not quit)
    let release_event = crossterm::event::KeyEvent {
        code: KeyCode::F(10),
        modifiers: KeyModifiers::empty(),
        kind: KeyEventKind::Release,
        state: crossterm::event::KeyEventState::empty(),
    };

    let mut events = vec![
        // Release event -- must be ignored (no quit, no action)
        Event::Key(release_event),
        // Real Press quit event
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
// Non-Key events (mouse, resize) -- hit the outer if-let guard
// -----------------------------------------------------------------------

#[test]
fn test_run_app_non_key_event_ignored() -> io::Result<()> {
    let temp_dir = TempDir::new().expect("test setup");

    let mut app = App::new().expect("test setup");
    app.left_panel = Panel::new(temp_dir.path().to_path_buf()).expect("test setup");
    app.right_panel = Panel::new(temp_dir.path().to_path_buf()).expect("test setup");

    let backend = WritableTestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).expect("test setup");

    let mut events = vec![
        // Resize event -- not a Key event, must fall through silently
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

// --- Key Release events should be ignored ---

#[test]
fn test_key_release_ignored() -> io::Result<()> {
    let temp_dir = TempDir::new().expect("test setup");
    let mut app = App::new().expect("test setup");
    app.left_panel = Panel::new(temp_dir.path().to_path_buf()).expect("test setup");
    app.right_panel = Panel::new(temp_dir.path().to_path_buf()).expect("test setup");

    let backend = WritableTestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).expect("test setup");

    let release_event = Event::Key(crossterm::event::KeyEvent {
        code: KeyCode::Down,
        modifiers: KeyModifiers::empty(),
        kind: KeyEventKind::Release,
        state: crossterm::event::KeyEventState::NONE,
    });

    let mut events = vec![release_event, Event::Key(key_event(KeyCode::F(10)))].into_iter();

    let result = run_app_with_event_reader(&mut terminal, app, || {
        events
            .next()
            .ok_or_else(|| io::Error::new(io::ErrorKind::UnexpectedEof, "No more events"))
    });

    assert!(result.is_ok());
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

// --- Ctrl+R refresh panels ---

#[test]
fn test_ctrl_r_refresh() -> io::Result<()> {
    let temp_dir = TempDir::new().expect("test setup");
    let mut app = App::new().expect("test setup");
    app.left_panel = Panel::new(temp_dir.path().to_path_buf()).expect("test setup");
    app.right_panel = Panel::new(temp_dir.path().to_path_buf()).expect("test setup");

    let backend = WritableTestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).expect("test setup");

    let mut events = vec![
        Event::Key(key_event_with_mods(
            KeyCode::Char('r'),
            KeyModifiers::CONTROL,
        )),
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
fn test_ctrl_r_refresh_renders_success_status() -> io::Result<()> {
    let temp_dir = TempDir::new().expect("test setup");
    let mut app = App::new().expect("test setup");
    app.left_panel = Panel::new(temp_dir.path().to_path_buf()).expect("test setup");
    app.right_panel = Panel::new(temp_dir.path().to_path_buf()).expect("test setup");

    let backend = WritableTestBackend::new(240, 24);
    let mut terminal = Terminal::new(backend).expect("test setup");

    let mut events = vec![
        Event::Key(key_event_with_mods(
            KeyCode::Char('r'),
            KeyModifiers::CONTROL,
        )),
        Event::Key(key_event(KeyCode::F(10))),
    ]
    .into_iter();

    run_app_with_event_reader(&mut terminal, app, || {
        events
            .next()
            .ok_or_else(|| io::Error::new(io::ErrorKind::UnexpectedEof, "No more events"))
    })?;

    let screen = buffer_text(&terminal);
    assert!(screen.contains("Panels refreshed"), "{screen}");
    Ok(())
}

#[test]
fn test_ctrl_r_refresh_renders_failure_status() -> io::Result<()> {
    let temp_dir = TempDir::new().expect("test setup");
    let left_dir = temp_dir.path().join("left");
    let right_dir = temp_dir.path().join("right");
    fs::create_dir(&left_dir).expect("test setup");
    fs::create_dir(&right_dir).expect("test setup");

    let mut app = App::new().expect("test setup");
    app.left_panel = Panel::new(left_dir.clone()).expect("test setup");
    app.right_panel = Panel::new(right_dir).expect("test setup");

    fs::remove_dir_all(&left_dir).expect("test setup");

    let backend = WritableTestBackend::new(240, 24);
    let mut terminal = Terminal::new(backend).expect("test setup");

    let mut events = vec![
        Event::Key(key_event_with_mods(
            KeyCode::Char('r'),
            KeyModifiers::CONTROL,
        )),
        Event::Key(key_event(KeyCode::F(10))),
    ]
    .into_iter();

    run_app_with_event_reader(&mut terminal, app, || {
        events
            .next()
            .ok_or_else(|| io::Error::new(io::ErrorKind::UnexpectedEof, "No more events"))
    })?;

    let screen = buffer_text(&terminal);
    assert!(screen.contains("Refresh failed"), "{screen}");
    Ok(())
}

// -----------------------------------------------------------------------
// F3 key opens the viewer for a file
// -----------------------------------------------------------------------

#[test]
fn test_f3_opens_viewer_on_selected_file() -> io::Result<()> {
    let temp_dir = TempDir::new().expect("test setup");
    fs::write(temp_dir.path().join("view_me.txt"), b"hello\nworld").expect("test setup");

    let mut app = App::new().expect("test setup");
    app.left_panel = Panel::new(temp_dir.path().to_path_buf()).expect("test setup");
    app.right_panel = Panel::new(temp_dir.path().to_path_buf()).expect("test setup");

    // Select the file.
    if let Some(idx) = app
        .left_panel
        .entries
        .iter()
        .position(|e| e.name == "view_me.txt")
    {
        app.left_panel.select(Some(idx));
    }

    let backend = WritableTestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).expect("test setup");

    // F3 opens the viewer; Esc closes it; then F10 exits.
    let mut events = vec![
        Event::Key(key_event(KeyCode::F(3))),
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
fn test_run_app_f3_renders_viewer_overlay() -> io::Result<()> {
    let temp_dir = TempDir::new().expect("test setup");
    fs::write(temp_dir.path().join("view_me.txt"), b"hello\nworld").expect("test setup");

    let mut app = App::new().expect("test setup");
    app.left_panel = Panel::new(temp_dir.path().to_path_buf()).expect("test setup");
    app.right_panel = Panel::new(temp_dir.path().to_path_buf()).expect("test setup");

    if let Some(idx) = app
        .left_panel
        .entries
        .iter()
        .position(|e| e.name == "view_me.txt")
    {
        app.left_panel.select(Some(idx));
    }

    let backend = WritableTestBackend::new(160, 30);
    let mut terminal = Terminal::new(backend).expect("test setup");

    let mut events = vec![Event::Key(key_event(KeyCode::F(3)))].into_iter();

    let err = run_app_with_event_reader(&mut terminal, app, || {
        events.next().ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::UnexpectedEof,
                "stop after viewer overlay renders",
            )
        })
    })
    .expect_err("event reader should stop after rendering viewer overlay");

    assert_eq!(err.kind(), io::ErrorKind::UnexpectedEof);
    let screen = buffer_text(&terminal);
    assert!(screen.contains("View: view_me.txt"), "{screen}");
    assert!(screen.contains("hello"), "{screen}");
    assert!(screen.contains("world"), "{screen}");
    Ok(())
}

#[test]
fn test_apply_action_toggle_help_and_console() -> io::Result<()> {
    let mut app = App::new()?;
    let backend = WritableTestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend)?;
    let mut last_nav = std::time::Instant::now();
    let nav_debounce = std::time::Duration::from_millis(0);

    apply_action(
        &mut terminal,
        &mut app,
        Action::ToggleHelp,
        &mut last_nav,
        nav_debounce,
    );
    assert!(matches!(app.overlay, Overlay::Help));

    apply_action(
        &mut terminal,
        &mut app,
        Action::ToggleHelp,
        &mut last_nav,
        nav_debounce,
    );
    assert!(matches!(app.overlay, Overlay::None));

    apply_action(
        &mut terminal,
        &mut app,
        Action::ToggleConsole,
        &mut last_nav,
        nav_debounce,
    );
    assert!(app.view == ViewMode::Console);

    apply_action(
        &mut terminal,
        &mut app,
        Action::ToggleConsole,
        &mut last_nav,
        nav_debounce,
    );
    assert!(matches!(app.overlay, Overlay::None));

    Ok(())
}
