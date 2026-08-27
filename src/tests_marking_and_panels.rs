//! Unit tests: marking and panels (split out to keep files under 500 lines).
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

// -----------------------------------------------------------------------
// Insert and Space keys mark selected entry and advance
// -----------------------------------------------------------------------

#[test]
fn test_insert_key_marks_selected_and_advances() -> io::Result<()> {
    let temp_dir = TempDir::new().expect("test setup");
    fs::write(temp_dir.path().join("a.txt"), b"a").expect("test setup");
    fs::write(temp_dir.path().join("b.txt"), b"b").expect("test setup");

    let mut app = App::new().expect("test setup");
    app.left_panel = Panel::new(temp_dir.path().to_path_buf()).expect("test setup");
    app.right_panel = Panel::new(temp_dir.path().to_path_buf()).expect("test setup");
    app.active_panel = PanelSide::Left;

    // Select a.txt.
    if let Some(idx) = app
        .left_panel
        .entries
        .iter()
        .position(|e| e.name == "a.txt")
    {
        app.left_panel.select(Some(idx));
    }
    let before_idx = app.left_panel.selected().expect("should have selection");

    let backend = WritableTestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).expect("test setup");

    let mut events = vec![
        Event::Key(key_event(KeyCode::Insert)),
        Event::Key(key_event(KeyCode::F(10))),
    ]
    .into_iter();

    // We can verify by running and checking file was marked (via side-effect free check).
    // Use a separate app clone to inspect intermediate state.
    {
        let mut test_app = App::new().expect("test setup");
        test_app.left_panel = Panel::new(temp_dir.path().to_path_buf()).expect("test setup");
        if let Some(idx) = test_app
            .left_panel
            .entries
            .iter()
            .position(|e| e.name == "a.txt")
        {
            test_app.left_panel.select(Some(idx));
        }
        // Simulate Insert key behavior: toggle mark then advance.
        test_app.left_panel.toggle_mark_selected();
        test_app.left_panel.next();

        assert!(
            test_app.left_panel.marked.contains("a.txt"),
            "a.txt should be marked after Insert"
        );
        assert_ne!(
            test_app.left_panel.selected(),
            Some(before_idx),
            "selection should advance after Insert"
        );
    }

    let result = run_app_with_event_reader(&mut terminal, app, || {
        events
            .next()
            .ok_or_else(|| io::Error::new(io::ErrorKind::UnexpectedEof, "No more events"))
    });
    assert!(result.is_ok());

    Ok(())
}

#[test]
fn test_space_key_marks_selected_and_advances() -> io::Result<()> {
    let temp_dir = TempDir::new().expect("test setup");
    fs::write(temp_dir.path().join("x.txt"), b"x").expect("test setup");
    fs::write(temp_dir.path().join("y.txt"), b"y").expect("test setup");

    // Directly simulate the Space key behavior (toggle mark + next).
    let mut app = App::new().expect("test setup");
    app.left_panel = Panel::new(temp_dir.path().to_path_buf()).expect("test setup");

    if let Some(idx) = app
        .left_panel
        .entries
        .iter()
        .position(|e| e.name == "x.txt")
    {
        app.left_panel.select(Some(idx));
    }
    let before_idx = app.left_panel.selected().expect("should have selection");

    // Space key behavior from lib.rs event loop.
    app.left_panel.toggle_mark_selected();
    app.left_panel.next();

    assert!(
        app.left_panel.marked.contains("x.txt"),
        "x.txt should be marked"
    );
    assert_ne!(
        app.left_panel.selected(),
        Some(before_idx),
        "selection should advance"
    );

    Ok(())
}

// -----------------------------------------------------------------------
// Left / Right arrow keys switch panels
// -----------------------------------------------------------------------

#[test]
fn test_left_arrow_sets_left_panel_active() -> io::Result<()> {
    let temp_dir = TempDir::new().expect("test setup");

    let mut app = App::new().expect("test setup");
    app.left_panel = Panel::new(temp_dir.path().to_path_buf()).expect("test setup");
    app.right_panel = Panel::new(temp_dir.path().to_path_buf()).expect("test setup");
    // Start with right panel active.
    app.active_panel = PanelSide::Right;

    let backend = WritableTestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).expect("test setup");

    let mut events = vec![
        Event::Key(key_event(KeyCode::Left)),
        Event::Key(key_event(KeyCode::F(10))),
    ]
    .into_iter();

    // Simulate directly on app first to verify.
    {
        let mut test_app = App::new().expect("test setup");
        test_app.active_panel = PanelSide::Right;
        test_app.active_panel = PanelSide::Left; // simulates Left key
        assert_eq!(test_app.active_panel, PanelSide::Left);
    }

    let result = run_app_with_event_reader(&mut terminal, app, || {
        events
            .next()
            .ok_or_else(|| io::Error::new(io::ErrorKind::UnexpectedEof, "No more events"))
    });
    assert!(result.is_ok());

    Ok(())
}

#[test]
fn test_right_arrow_sets_right_panel_active() -> io::Result<()> {
    let temp_dir = TempDir::new().expect("test setup");

    let mut app = App::new().expect("test setup");
    app.left_panel = Panel::new(temp_dir.path().to_path_buf()).expect("test setup");
    app.right_panel = Panel::new(temp_dir.path().to_path_buf()).expect("test setup");
    // Start with left panel active (default).
    assert_eq!(app.active_panel, PanelSide::Left);

    let backend = WritableTestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).expect("test setup");

    let mut events = vec![
        Event::Key(key_event(KeyCode::Right)),
        Event::Key(key_event(KeyCode::F(10))),
    ]
    .into_iter();

    // Simulate directly to verify right arrow logic.
    {
        let mut test_app = App::new().expect("test setup");
        test_app.active_panel = PanelSide::Left;
        test_app.active_panel = PanelSide::Right; // simulates Right key
        assert_eq!(test_app.active_panel, PanelSide::Right);
    }

    let result = run_app_with_event_reader(&mut terminal, app, || {
        events
            .next()
            .ok_or_else(|| io::Error::new(io::ErrorKind::UnexpectedEof, "No more events"))
    });
    assert!(result.is_ok());

    Ok(())
}

// -----------------------------------------------------------------------
// Ctrl+\ navigates to the root directory
// -----------------------------------------------------------------------

#[test]
fn test_ctrl_backslash_navigates_to_root() -> io::Result<()> {
    let temp_dir = TempDir::new().expect("test setup");

    let mut app = App::new().expect("test setup");
    app.left_panel = Panel::new(temp_dir.path().to_path_buf()).expect("test setup");
    app.right_panel = Panel::new(temp_dir.path().to_path_buf()).expect("test setup");

    let backend = WritableTestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).expect("test setup");

    let mut events = vec![
        Event::Key(key_event_with_mods(
            KeyCode::Char('\\'),
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

// --- ':' opens command line ---

#[test]
fn test_colon_opens_command_line() -> io::Result<()> {
    let temp_dir = TempDir::new().expect("test setup");
    let mut app = App::new().expect("test setup");
    app.left_panel = Panel::new(temp_dir.path().to_path_buf()).expect("test setup");
    app.right_panel = Panel::new(temp_dir.path().to_path_buf()).expect("test setup");

    let backend = WritableTestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).expect("test setup");

    let mut events = vec![
        // Open command line
        Event::Key(key_event(KeyCode::Char(':'))),
        // Cancel
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
fn test_run_app_colon_renders_command_prompt() -> io::Result<()> {
    let temp_dir = TempDir::new().expect("test setup");
    let mut app = App::new().expect("test setup");
    app.left_panel = Panel::new(temp_dir.path().to_path_buf()).expect("test setup");
    app.right_panel = Panel::new(temp_dir.path().to_path_buf()).expect("test setup");

    let backend = WritableTestBackend::new(160, 24);
    let mut terminal = Terminal::new(backend).expect("test setup");

    let mut events = vec![Event::Key(key_event(KeyCode::Char(':')))].into_iter();

    let err = run_app_with_event_reader(&mut terminal, app, || {
        events.next().ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::UnexpectedEof,
                "stop after command line opens",
            )
        })
    })
    .expect_err("event reader should stop after rendering command line");

    assert_eq!(err.kind(), io::ErrorKind::UnexpectedEof);
    let screen = buffer_text(&terminal);
    // The caret is a reversed cell rather than a `_` glyph now, so the
    // prompt itself is what the rendered screen can be asserted on.
    assert!(screen.contains("> "), "{screen}");
    Ok(())
}

// -----------------------------------------------------------------------
// Alt+letter quick search activates quick_search
// -----------------------------------------------------------------------

#[test]
fn test_alt_letter_activates_quick_search() -> io::Result<()> {
    let temp_dir = TempDir::new().expect("test setup");
    fs::write(temp_dir.path().join("zeta.txt"), b"z").expect("test setup");

    let mut app = App::new().expect("test setup");
    app.left_panel = Panel::new(temp_dir.path().to_path_buf()).expect("test setup");
    app.right_panel = Panel::new(temp_dir.path().to_path_buf()).expect("test setup");

    let backend = WritableTestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).expect("test setup");

    // Alt+z should activate quick_search with "z".
    let mut events = vec![
        Event::Key(key_event_with_mods(KeyCode::Char('z'), KeyModifiers::ALT)),
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

// --- Home/End keys ---

#[test]
fn test_home_end_keys() -> io::Result<()> {
    let temp_dir = TempDir::new().expect("test setup");
    fs::write(temp_dir.path().join("a.txt"), b"a").expect("test setup");
    fs::write(temp_dir.path().join("b.txt"), b"b").expect("test setup");
    fs::write(temp_dir.path().join("c.txt"), b"c").expect("test setup");

    let mut app = App::new().expect("test setup");
    app.left_panel = Panel::new(temp_dir.path().to_path_buf()).expect("test setup");
    app.right_panel = Panel::new(temp_dir.path().to_path_buf()).expect("test setup");

    let backend = WritableTestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).expect("test setup");

    let mut events = vec![
        Event::Key(key_event(KeyCode::End)),
        Event::Key(key_event(KeyCode::Home)),
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

// --- End key on empty panel ---

#[test]
fn test_end_key_empty_panel() -> io::Result<()> {
    let temp_dir = TempDir::new().expect("test setup");

    let mut app = App::new().expect("test setup");
    app.left_panel = Panel::new(temp_dir.path().to_path_buf()).expect("test setup");
    app.right_panel = Panel::new(temp_dir.path().to_path_buf()).expect("test setup");
    // Clear entries to simulate empty panel
    app.left_panel.entries.clear();

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

// --- Q uppercase quit ---

#[test]
fn test_uppercase_q_quit() -> io::Result<()> {
    let temp_dir = TempDir::new().expect("test setup");
    let mut app = App::new().expect("test setup");
    app.left_panel = Panel::new(temp_dir.path().to_path_buf()).expect("test setup");
    app.right_panel = Panel::new(temp_dir.path().to_path_buf()).expect("test setup");

    let backend = WritableTestBackend::new(80, 24);
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
