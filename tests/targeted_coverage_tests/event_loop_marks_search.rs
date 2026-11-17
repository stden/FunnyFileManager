//! Purpose: Marking and quick search through the event loop.

use super::*;

#[test]
fn test_event_loop_backspace_quick_search() -> io::Result<()> {
    use crossterm::event::Event;

    let temp = TempDir::new()?;
    File::create(temp.path().join("alpha.txt"))?.write_all(b"a")?;
    File::create(temp.path().join("beta.txt"))?.write_all(b"b")?;

    let mut app = App::new()?;
    app.left_panel.path = temp.path().to_path_buf();
    app.left_panel.load_entries()?;

    run_event_sequence(
        app,
        vec![
            Event::Key(KeyEvent::new(KeyCode::Char('b'), KeyModifiers::ALT)),
            Event::Key(KeyEvent::new(KeyCode::Char('e'), KeyModifiers::ALT)),
            Event::Key(key(KeyCode::Backspace)),
            Event::Key(key(KeyCode::Backspace)),
            Event::Key(key(KeyCode::F(10))),
            // F10 now asks before quitting; Enter answers Yes.
            Event::Key(key(KeyCode::Enter)),
        ],
    )
}

#[test]
fn test_event_loop_insert_mark() -> io::Result<()> {
    use crossterm::event::Event;

    let temp = TempDir::new()?;
    File::create(temp.path().join("file.txt"))?.write_all(b"x")?;

    let mut app = App::new()?;
    app.left_panel.path = temp.path().to_path_buf();
    app.left_panel.load_entries()?;

    // Select file.txt
    for (i, entry) in app.left_panel.entries.iter().enumerate() {
        if entry.name == "file.txt" {
            app.left_panel.select(Some(i));
            break;
        }
    }

    run_event_sequence(
        app,
        vec![
            Event::Key(key(KeyCode::Insert)),
            Event::Key(key(KeyCode::F(10))),
            // F10 now asks before quitting; Enter answers Yes.
            Event::Key(key(KeyCode::Enter)),
        ],
    )
}

#[test]
fn test_event_loop_space_toggles_mark() -> io::Result<()> {
    use crossterm::event::Event;

    let temp = TempDir::new()?;
    File::create(temp.path().join("x.txt"))?.write_all(b"x")?;

    let mut app = App::new()?;
    app.left_panel.path = temp.path().to_path_buf();
    app.left_panel.load_entries()?;
    for (i, entry) in app.left_panel.entries.iter().enumerate() {
        if entry.name == "x.txt" {
            app.left_panel.select(Some(i));
            break;
        }
    }

    run_event_sequence(
        app,
        vec![
            Event::Key(key(KeyCode::Char(' '))),
            Event::Key(key(KeyCode::F(10))),
            // F10 now asks before quitting; Enter answers Yes.
            Event::Key(key(KeyCode::Enter)),
        ],
    )
}

#[test]
fn test_event_loop_esc_clears_quick_search() -> io::Result<()> {
    use crossterm::event::Event;

    let app = App::new()?;
    run_event_sequence(
        app,
        vec![
            Event::Key(KeyEvent::new(KeyCode::Char('a'), KeyModifiers::ALT)),
            Event::Key(key(KeyCode::Esc)),
            Event::Key(key(KeyCode::F(10))),
            // F10 now asks before quitting; Enter answers Yes.
            Event::Key(key(KeyCode::Enter)),
        ],
    )
}
