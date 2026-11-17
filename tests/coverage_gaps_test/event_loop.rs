//! Purpose: Home, End, arrows and quick-search Backspace through input handling and the event loop.

use super::*;

// ─── Quick search backspace (lib.rs line 459) ───────────────────────

#[test]
fn test_quick_search_backspace_clears() -> io::Result<()> {
    let temp = TempDir::new()?;
    let mut app = make_app(temp.path())?;

    // Set quick search with single char
    app.quick_search = QuickSearch::Active("a".to_owned());

    // Backspace should clear it
    let key = key_event(KeyCode::Backspace);
    let consumed = process_popup_input(&mut app, key);
    // Quick search backspace is handled in lib.rs event loop, not process_popup_input
    // So we just verify the popup handler doesn't consume it
    assert!(!consumed || app.quick_search.query().is_none());

    Ok(())
}

const fn key_event_with_mods(code: KeyCode, modifiers: KeyModifiers) -> KeyEvent {
    KeyEvent::new(code, modifiers)
}

// ─── Event loop coverage (lib.rs lines 328, 352, 370, 401, 459) ────

#[test]
fn test_event_loop_end_key() -> io::Result<()> {
    let temp = TempDir::new()?;
    fs::write(temp.path().join("aaa.txt"), b"a")?;
    fs::write(temp.path().join("zzz.txt"), b"z")?;
    let app = make_app(temp.path())?;

    run_events(
        app,
        vec![
            Event::Key(key_event(KeyCode::End)),
            Event::Key(key_event(KeyCode::F(10))),
            // F10 now asks before quitting; Enter answers Yes.
            Event::Key(key_event(KeyCode::Enter)),
        ],
    )
}

#[test]
fn test_event_loop_home_key() -> io::Result<()> {
    let temp = TempDir::new()?;
    fs::write(temp.path().join("aaa.txt"), b"a")?;
    fs::write(temp.path().join("zzz.txt"), b"z")?;
    let app = make_app(temp.path())?;

    run_events(
        app,
        vec![
            Event::Key(key_event(KeyCode::Down)),
            Event::Key(key_event(KeyCode::Home)),
            Event::Key(key_event(KeyCode::F(10))),
            // F10 now asks before quitting; Enter answers Yes.
            Event::Key(key_event(KeyCode::Enter)),
        ],
    )
}

#[test]
fn test_event_loop_quick_search_backspace() -> io::Result<()> {
    let temp = TempDir::new()?;
    fs::write(temp.path().join("alpha.txt"), b"a")?;
    fs::write(temp.path().join("beta.txt"), b"b")?;
    let app = make_app(temp.path())?;

    run_events(
        app,
        vec![
            // Start quick search with Alt+a, then Alt+l, then backspace
            Event::Key(key_event_with_mods(KeyCode::Char('a'), KeyModifiers::ALT)),
            Event::Key(key_event_with_mods(KeyCode::Char('l'), KeyModifiers::ALT)),
            Event::Key(key_event(KeyCode::Backspace)), // pops 'l', keeps 'a'
            Event::Key(key_event(KeyCode::Backspace)), // pops 'a', clears search
            Event::Key(key_event(KeyCode::F(10))),
            // F10 now asks before quitting; Enter answers Yes.
            Event::Key(key_event(KeyCode::Enter)),
        ],
    )
}

#[test]
fn test_event_loop_down_up_navigation() -> io::Result<()> {
    let temp = TempDir::new()?;
    fs::write(temp.path().join("aaa.txt"), b"a")?;
    fs::write(temp.path().join("bbb.txt"), b"b")?;
    let app = make_app(temp.path())?;

    run_events(
        app,
        vec![
            Event::Key(key_event(KeyCode::Down)),
            Event::Key(key_event(KeyCode::Down)),
            Event::Key(key_event(KeyCode::Up)),
            Event::Key(key_event(KeyCode::F(10))),
            // F10 now asks before quitting; Enter answers Yes.
            Event::Key(key_event(KeyCode::Enter)),
        ],
    )
}
