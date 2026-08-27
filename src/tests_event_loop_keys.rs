//! Unit tests: event loop keys (split out to keep files under 500 lines).
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

    let mut events = vec![
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
        // F5 copy (select file first)
        Event::Key(key_event(KeyCode::Down)),
        Event::Key(key_event(KeyCode::F(5))),
        // F6 move (cancel via Esc since no confirmation needed)
        Event::Key(key_event(KeyCode::F(6))),
        // F8 delete (cancel)
        Event::Key(key_event(KeyCode::F(8))),
        // Quit
        Event::Key(key_event(KeyCode::F(10))),
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
fn test_navigation_moves_one_item() -> io::Result<()> {
    let temp_dir = TempDir::new().expect("test setup");
    // Create multiple files to test navigation
    for i in 1..=5 {
        fs::write(temp_dir.path().join(format!("file{i}.txt")), b"data").expect("test setup");
    }

    let mut app = App::new().expect("test setup");
    app.left_panel = Panel::new(temp_dir.path().to_path_buf()).expect("test setup");

    // Start at first item
    app.left_panel.select(Some(0));

    // Move down once
    app.left_panel.next();
    assert_eq!(
        app.left_panel.selected(),
        Some(1),
        "Should move to second item"
    );

    // Move down again
    app.left_panel.next();
    assert_eq!(
        app.left_panel.selected(),
        Some(2),
        "Should move to third item"
    );

    // Move up once
    app.left_panel.previous();
    assert_eq!(
        app.left_panel.selected(),
        Some(1),
        "Should move back to second item"
    );

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
fn test_quick_search_alt_key() -> io::Result<()> {
    let temp_dir = TempDir::new().expect("test setup");
    fs::write(temp_dir.path().join("alpha.txt"), b"data").expect("test setup");
    fs::write(temp_dir.path().join("beta.txt"), b"data").expect("test setup");
    fs::write(temp_dir.path().join("gamma.txt"), b"data").expect("test setup");

    let mut app = App::new().expect("test setup");
    app.left_panel = Panel::new(temp_dir.path().to_path_buf()).expect("test setup");
    app.right_panel = Panel::new(temp_dir.path().to_path_buf()).expect("test setup");

    let backend = WritableTestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).expect("test setup");

    let mut events = vec![
        // Alt+b to search for "beta"
        Event::Key(key_event_with_mods(KeyCode::Char('b'), KeyModifiers::ALT)),
        // Esc to clear search
        Event::Key(key_event(KeyCode::Esc)),
        // Alt+g to search for "gamma"
        Event::Key(key_event_with_mods(KeyCode::Char('g'), KeyModifiers::ALT)),
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
fn test_quick_search_multi_char() -> io::Result<()> {
    let temp_dir = TempDir::new().expect("test setup");
    fs::write(temp_dir.path().join("apple.txt"), b"data").expect("test setup");
    fs::write(temp_dir.path().join("apricot.txt"), b"data").expect("test setup");
    fs::write(temp_dir.path().join("avocado.txt"), b"data").expect("test setup");

    let mut app = App::new().expect("test setup");
    app.left_panel = Panel::new(temp_dir.path().to_path_buf()).expect("test setup");
    app.right_panel = Panel::new(temp_dir.path().to_path_buf()).expect("test setup");

    let backend = WritableTestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).expect("test setup");

    let mut events = vec![
        // Type "ap" to narrow search to apple/apricot
        Event::Key(key_event_with_mods(KeyCode::Char('a'), KeyModifiers::ALT)),
        Event::Key(key_event_with_mods(KeyCode::Char('p'), KeyModifiers::ALT)),
        // Backspace to remove 'p'
        Event::Key(key_event(KeyCode::Backspace)),
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
fn test_quick_search_backspace_clears() -> io::Result<()> {
    let temp_dir = TempDir::new().expect("test setup");
    fs::write(temp_dir.path().join("test.txt"), b"data").expect("test setup");

    let mut app = App::new().expect("test setup");
    app.left_panel = Panel::new(temp_dir.path().to_path_buf()).expect("test setup");
    app.right_panel = Panel::new(temp_dir.path().to_path_buf()).expect("test setup");

    let backend = WritableTestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).expect("test setup");

    let mut events = vec![
        // Start quick search
        Event::Key(key_event_with_mods(KeyCode::Char('t'), KeyModifiers::ALT)),
        // Backspace clears the single character, clearing quick search
        Event::Key(key_event(KeyCode::Backspace)),
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
fn test_quick_search_no_match_fallback() -> io::Result<()> {
    let temp_dir = TempDir::new().expect("test setup");
    fs::write(temp_dir.path().join("apple.txt"), b"data").expect("test setup");
    fs::write(temp_dir.path().join("banana.txt"), b"data").expect("test setup");

    let mut app = App::new().expect("test setup");
    app.left_panel = Panel::new(temp_dir.path().to_path_buf()).expect("test setup");
    app.right_panel = Panel::new(temp_dir.path().to_path_buf()).expect("test setup");

    let backend = WritableTestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).expect("test setup");

    let mut events = vec![
        // Search for "a"
        Event::Key(key_event_with_mods(KeyCode::Char('a'), KeyModifiers::ALT)),
        // Then "x" - no match for "ax", should try just "x"
        Event::Key(key_event_with_mods(KeyCode::Char('x'), KeyModifiers::ALT)),
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
fn test_quick_search_no_match_single_char() -> io::Result<()> {
    let temp_dir = TempDir::new().expect("test setup");
    fs::write(temp_dir.path().join("apple.txt"), b"data").expect("test setup");

    let mut app = App::new().expect("test setup");
    app.left_panel = Panel::new(temp_dir.path().to_path_buf()).expect("test setup");
    app.right_panel = Panel::new(temp_dir.path().to_path_buf()).expect("test setup");

    let backend = WritableTestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).expect("test setup");

    let mut events = vec![
        // No match for "x" should exercise the single-char fallback path
        Event::Key(key_event_with_mods(KeyCode::Char('x'), KeyModifiers::ALT)),
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
fn test_apply_action_pages_selection_by_one_screen() -> io::Result<()> {
    let temp_dir = TempDir::new()?;
    for i in 0..40 {
        fs::write(temp_dir.path().join(format!("file{i:03}.txt")), b"x")?;
    }

    let mut app = App::new()?;
    app.left_panel = Panel::new(temp_dir.path().to_path_buf())?;
    app.active_panel = PanelSide::Left;
    app.left_panel.select(Some(0));

    // 80x24 terminal => 24 - 4 chrome - 2 borders = 18 visible rows.
    let backend = WritableTestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend)?;
    let mut last_nav = std::time::Instant::now();
    let nav_debounce = std::time::Duration::from_millis(0);

    apply_action(
        &mut terminal,
        &mut app,
        Action::PageDown,
        &mut last_nav,
        nav_debounce,
    );
    assert_eq!(app.left_panel.selected(), Some(18));

    apply_action(
        &mut terminal,
        &mut app,
        Action::PageUp,
        &mut last_nav,
        nav_debounce,
    );
    assert_eq!(app.left_panel.selected(), Some(0));

    Ok(())
}

/// Typing in the panels starts a command, the way Far Manager and every shell
/// do — the character is not swallowed, and it is not a quick search either
/// (that stayed on Alt+letter).
#[test]
fn a_printable_key_opens_the_command_line_holding_it() -> io::Result<()> {
    let temp_dir = TempDir::new().expect("test setup");
    let mut app = App::new().expect("test setup");
    app.left_panel = Panel::new(temp_dir.path().to_path_buf()).expect("test setup");

    let backend = WritableTestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).expect("test setup");
    let mut last_nav_time = std::time::Instant::now();

    for ch in ['l', 's'] {
        dispatch_key(
            &mut terminal,
            &mut app,
            KeyEvent::new(KeyCode::Char(ch), KeyModifiers::empty()),
            &mut last_nav_time,
            std::time::Duration::ZERO,
        );
    }

    assert_eq!(app.get_command_line(), Some("ls"));
    assert!(
        app.quick_search.query().is_none(),
        "typing is not a quick search"
    );
    Ok(())
}

/// `q` is the first letter of `quit`, `qemu` and `queue`. It has to be
/// typeable, so it starts a command instead of ending the program; F10 still
/// quits, and so does the `quit` command it can now spell.
#[test]
fn bare_q_types_instead_of_quitting() -> io::Result<()> {
    let temp_dir = TempDir::new().expect("test setup");
    let mut app = App::new().expect("test setup");
    app.left_panel = Panel::new(temp_dir.path().to_path_buf()).expect("test setup");

    let backend = WritableTestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).expect("test setup");
    let mut last_nav_time = std::time::Instant::now();

    dispatch_key(
        &mut terminal,
        &mut app,
        KeyEvent::new(KeyCode::Char('q'), KeyModifiers::empty()),
        &mut last_nav_time,
        std::time::Duration::ZERO,
    );

    assert!(app.lifecycle.is_running(), "q must not quit");
    assert_eq!(app.get_command_line(), Some("q"));
    assert!(app.lifecycle.is_running());
    Ok(())
}
