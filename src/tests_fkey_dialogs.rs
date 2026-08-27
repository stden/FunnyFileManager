//! Unit tests: fkey dialogs (split out to keep files under 500 lines).
#![allow(unused_imports)]

use super::tests::*;
use crate::models::BookmarkSlot;

use super::*;

use crate::models::{Panel, PanelSide};

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use std::env;

use std::fs;

use std::path::{Path, PathBuf};

use std::sync::{Mutex, OnceLock};

use tempfile::TempDir;

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

// --- Ctrl+O console mode ---

#[test]
fn test_ctrl_o_console_mode_toggle() -> io::Result<()> {
    let temp_dir = TempDir::new().expect("test setup");
    let mut app = App::new().expect("test setup");
    app.left_panel = Panel::new(temp_dir.path().to_path_buf()).expect("test setup");
    app.right_panel = Panel::new(temp_dir.path().to_path_buf()).expect("test setup");

    let backend = WritableTestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).expect("test setup");

    let mut events = vec![
        // Toggle console on
        Event::Key(key_event_with_mods(
            KeyCode::Char('o'),
            KeyModifiers::CONTROL,
        )),
        // Toggle console off
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

// --- Ctrl+U swap panels ---

#[test]
fn test_ctrl_u_swap_panels() -> io::Result<()> {
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
        Event::Key(key_event_with_mods(
            KeyCode::Char('u'),
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

// -----------------------------------------------------------------------
// F5 / F6 / F7 / F8 -- verify the handler functions run without panic
// -----------------------------------------------------------------------

#[test]
fn test_f5_copy_key_in_event_loop() -> io::Result<()> {
    let temp_dir = TempDir::new().expect("test setup");
    let left_dir = temp_dir.path().join("left");
    let right_dir = temp_dir.path().join("right");
    fs::create_dir(&left_dir).expect("test setup");
    fs::create_dir(&right_dir).expect("test setup");
    fs::write(left_dir.join("copy_me.txt"), b"data").expect("test setup");

    let mut app = App::new().expect("test setup");
    app.left_panel = Panel::new(left_dir).expect("test setup");
    app.right_panel = Panel::new(right_dir.clone()).expect("test setup");
    app.active_panel = PanelSide::Left;

    // Select the file.
    if let Some(idx) = app
        .left_panel
        .entries
        .iter()
        .position(|e| e.name == "copy_me.txt")
    {
        app.left_panel.select(Some(idx));
    }

    let backend = WritableTestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).expect("test setup");

    let mut events = vec![
        Event::Key(key_event(KeyCode::F(5))),
        Event::Key(key_event(KeyCode::F(10))),
    ]
    .into_iter();

    let result = run_app_with_event_reader(&mut terminal, app, || {
        events
            .next()
            .ok_or_else(|| io::Error::new(io::ErrorKind::UnexpectedEof, "No more events"))
    });

    assert!(result.is_ok());
    assert!(
        right_dir.join("copy_me.txt").exists(),
        "file should have been copied"
    );

    Ok(())
}

#[test]
fn test_f6_move_key_in_event_loop() -> io::Result<()> {
    let temp_dir = TempDir::new().expect("test setup");
    let left_dir = temp_dir.path().join("left_f6");
    let right_dir = temp_dir.path().join("right_f6");
    fs::create_dir(&left_dir).expect("test setup");
    fs::create_dir(&right_dir).expect("test setup");
    fs::write(left_dir.join("move_me.txt"), b"move").expect("test setup");

    let mut app = App::new().expect("test setup");
    app.left_panel = Panel::new(left_dir.clone()).expect("test setup");
    app.right_panel = Panel::new(right_dir.clone()).expect("test setup");
    app.active_panel = PanelSide::Left;

    if let Some(idx) = app
        .left_panel
        .entries
        .iter()
        .position(|e| e.name == "move_me.txt")
    {
        app.left_panel.select(Some(idx));
    }

    let backend = WritableTestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).expect("test setup");

    let mut events = vec![
        Event::Key(key_event(KeyCode::F(6))),
        Event::Key(key_event(KeyCode::F(10))),
    ]
    .into_iter();

    let result = run_app_with_event_reader(&mut terminal, app, || {
        events
            .next()
            .ok_or_else(|| io::Error::new(io::ErrorKind::UnexpectedEof, "No more events"))
    });

    assert!(result.is_ok());
    assert!(
        !left_dir.join("move_me.txt").exists(),
        "file should have been moved away"
    );
    assert!(
        right_dir.join("move_me.txt").exists(),
        "file should be at destination"
    );

    Ok(())
}

#[test]
fn test_f7_mkdir_key_in_event_loop() -> io::Result<()> {
    let temp_dir = TempDir::new().expect("test setup");

    let mut app = App::new().expect("test setup");
    app.left_panel = Panel::new(temp_dir.path().to_path_buf()).expect("test setup");
    app.right_panel = Panel::new(temp_dir.path().to_path_buf()).expect("test setup");

    let backend = WritableTestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).expect("test setup");

    let mut events = vec![
        Event::Key(key_event(KeyCode::F(7))),
        Event::Key(key_event(KeyCode::F(10))),
    ]
    .into_iter();

    let result = run_app_with_event_reader(&mut terminal, app, || {
        events
            .next()
            .ok_or_else(|| io::Error::new(io::ErrorKind::UnexpectedEof, "No more events"))
    });

    assert!(result.is_ok());
    assert!(
        temp_dir.path().join("new_folder").exists(),
        "new_folder should have been created"
    );

    Ok(())
}

#[test]
fn test_f8_delete_confirm_then_y_deletes_file() -> io::Result<()> {
    let temp_dir = TempDir::new().expect("test setup");
    let file_path = temp_dir.path().join("del_via_f8.txt");
    fs::write(&file_path, b"bye").expect("test setup");

    let mut app = App::new().expect("test setup");
    app.left_panel = Panel::new(temp_dir.path().to_path_buf()).expect("test setup");
    app.right_panel = Panel::new(temp_dir.path().to_path_buf()).expect("test setup");
    app.active_panel = PanelSide::Left;

    if let Some(idx) = app
        .left_panel
        .entries
        .iter()
        .position(|e| e.name == "del_via_f8.txt")
    {
        app.left_panel.select(Some(idx));
    }

    let backend = WritableTestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).expect("test setup");

    let mut events = vec![
        // F8 sets up delete_confirm overlay.
        Event::Key(key_event(KeyCode::F(8))),
        // 'y' confirms deletion.
        Event::Key(key_event(KeyCode::Char('y'))),
        Event::Key(key_event(KeyCode::F(10))),
    ]
    .into_iter();

    let result = run_app_with_event_reader(&mut terminal, app, || {
        events
            .next()
            .ok_or_else(|| io::Error::new(io::ErrorKind::UnexpectedEof, "No more events"))
    });

    assert!(result.is_ok());
    assert!(!file_path.exists(), "file should have been deleted");

    Ok(())
}

// --- Bookmarks live on the `:` command line, not on an overlay ---

/// Plain F2 is deliberately unbound. FFM never had a bookmark or search overlay
/// that a key could reach, so F2 must fall through to the panel handler; a modal
/// here would swallow every key while `render_overlays` drew nothing, leaving the
/// app looking frozen.
#[test]
fn test_f2_opens_no_overlay() -> io::Result<()> {
    let temp_dir = TempDir::new().expect("test setup");
    let mut app = App::new().expect("test setup");
    app.left_panel = Panel::new(temp_dir.path().to_path_buf()).expect("test setup");
    app.right_panel = Panel::new(temp_dir.path().to_path_buf()).expect("test setup");

    let backend = WritableTestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).expect("test setup");

    let mut events = vec![
        Event::Key(key_event(KeyCode::F(2))),
        Event::Key(key_event(KeyCode::F(10))),
    ]
    .into_iter();

    let result = run_app_with_event_reader(&mut terminal, app, || {
        events
            .next()
            .ok_or_else(|| io::Error::new(io::ErrorKind::UnexpectedEof, "No more events"))
    });

    // F10 quit means no overlay consumed it — an open modal returns true from
    // `process_popup_input` and the reader would have run dry with UnexpectedEof.
    assert!(result.is_ok(), "F2 must not open a key-swallowing overlay");
    Ok(())
}

/// The `:` prompt is the only path that ever set or opened a bookmark; keep the
/// end-to-end keystroke route covered so the deleted overlay is not missed.
#[test]
fn test_bookmark_roundtrip_via_command_line() -> io::Result<()> {
    let temp_dir = TempDir::new().expect("test setup");
    let target = temp_dir.path().join("target");
    fs::create_dir(&target).expect("test setup");

    let mut app = App::new().expect("test setup");
    app.left_panel = Panel::new(target.clone()).expect("test setup");
    app.right_panel = Panel::new(temp_dir.path().to_path_buf()).expect("test setup");

    let backend = WritableTestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).expect("test setup");

    let mut events = vec![Event::Key(key_event(KeyCode::Char(':')))];
    events.extend(
        "bookmark set 3"
            .chars()
            .map(|c| Event::Key(key_event(KeyCode::Char(c)))),
    );
    events.push(Event::Key(key_event(KeyCode::Enter)));
    events.push(Event::Key(key_event(KeyCode::F(10))));
    let mut events = events.into_iter();

    run_app_with_event_reader(&mut terminal, app, || {
        events
            .next()
            .ok_or_else(|| io::Error::new(io::ErrorKind::UnexpectedEof, "No more events"))
    })
    .expect("event loop");

    // The loop consumed the App; re-run the same command directly to assert the
    // slot it writes, which is what `bookmark go` reads back.
    let mut app = App::new().expect("test setup");
    app.left_panel = Panel::new(target.clone()).expect("test setup");
    crate::handlers::execute_command(&mut app, "bookmark set 3").expect("bookmark set 3");
    assert_eq!(
        app.bookmarks
            .get(BookmarkSlot::from_usize(3).expect("slot")),
        Some(target.as_path())
    );

    app.left_panel = Panel::new(temp_dir.path().to_path_buf()).expect("test setup");
    crate::handlers::execute_command(&mut app, "bookmark go 3").expect("bookmark go 3");
    assert_eq!(app.left_panel.path, target);
    Ok(())
}
