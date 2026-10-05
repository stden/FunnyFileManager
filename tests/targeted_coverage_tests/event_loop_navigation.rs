//! Purpose: Cursor and directory keys through the event loop.

use super::*;

#[test]
fn test_event_loop_end_key() -> io::Result<()> {
    use crossterm::event::Event;

    let temp = TempDir::new()?;
    File::create(temp.path().join("a.txt"))?.write_all(b"a")?;
    File::create(temp.path().join("b.txt"))?.write_all(b"b")?;
    File::create(temp.path().join("c.txt"))?.write_all(b"c")?;

    let mut app = App::new()?;
    app.left_panel.path = temp.path().to_path_buf();
    app.left_panel.load_entries()?;
    app.left_panel.select(Some(0));

    run_event_sequence(
        app,
        vec![
            Event::Key(key(KeyCode::End)),
            Event::Key(key(KeyCode::F(10))),
            // F10 now asks before quitting; Enter answers Yes.
            Event::Key(key(KeyCode::Enter)),
        ],
    )
}

#[test]
fn test_event_loop_ctrl_backslash_root() -> io::Result<()> {
    use crossterm::event::Event;

    let app = App::new()?;
    run_event_sequence(
        app,
        vec![
            Event::Key(key_ctrl(KeyCode::Char('\\'))),
            Event::Key(key(KeyCode::F(10))),
            // F10 now asks before quitting; Enter answers Yes.
            Event::Key(key(KeyCode::Enter)),
        ],
    )
}

#[test]
fn test_event_loop_tab_and_panel_keys() -> io::Result<()> {
    use crossterm::event::Event;

    let app = App::new()?;
    run_event_sequence(
        app,
        vec![
            Event::Key(key(KeyCode::Tab)),
            Event::Key(key(KeyCode::Right)),
            Event::Key(key(KeyCode::Left)),
            Event::Key(key(KeyCode::F(10))),
            // F10 now asks before quitting; Enter answers Yes.
            Event::Key(key(KeyCode::Enter)),
        ],
    )
}

#[test]
fn test_event_loop_resize() -> io::Result<()> {
    use crossterm::event::Event;

    let app = App::new()?;
    run_event_sequence(
        app,
        vec![Event::Resize(100, 50), Event::Key(key(KeyCode::F(10)))],
    )
}

#[test]
fn test_event_loop_home_key() -> io::Result<()> {
    use crossterm::event::Event;

    let temp = TempDir::new()?;
    File::create(temp.path().join("a.txt"))?.write_all(b"a")?;
    File::create(temp.path().join("b.txt"))?.write_all(b"b")?;

    let mut app = App::new()?;
    app.left_panel.path = temp.path().to_path_buf();
    app.left_panel.load_entries()?;

    run_event_sequence(
        app,
        vec![
            Event::Key(key(KeyCode::End)),
            Event::Key(key(KeyCode::Home)),
            Event::Key(key(KeyCode::F(10))),
            // F10 now asks before quitting; Enter answers Yes.
            Event::Key(key(KeyCode::Enter)),
        ],
    )
}

#[test]
fn test_event_loop_enter_directory() -> io::Result<()> {
    use crossterm::event::Event;

    let temp = TempDir::new()?;
    fs::create_dir(temp.path().join("subdir"))?;

    let mut app = App::new()?;
    app.left_panel.path = temp.path().to_path_buf();
    app.left_panel.load_entries()?;

    // Select subdir
    for (i, entry) in app.left_panel.entries.iter().enumerate() {
        if entry.name == "subdir" {
            app.left_panel.select(Some(i));
            break;
        }
    }

    run_event_sequence(
        app,
        vec![
            Event::Key(key(KeyCode::Enter)),
            Event::Key(key(KeyCode::F(10))),
            // F10 now asks before quitting; Enter answers Yes.
            Event::Key(key(KeyCode::Enter)),
        ],
    )
}

#[test]
fn test_event_loop_down_up_navigation() -> io::Result<()> {
    use crossterm::event::Event;

    let temp = TempDir::new()?;
    File::create(temp.path().join("a.txt"))?.write_all(b"a")?;
    File::create(temp.path().join("b.txt"))?.write_all(b"b")?;

    let mut app = App::new()?;
    app.left_panel.path = temp.path().to_path_buf();
    app.left_panel.load_entries()?;

    run_event_sequence(
        app,
        vec![
            Event::Key(key(KeyCode::Down)),
            Event::Key(key(KeyCode::Down)),
            Event::Key(key(KeyCode::Up)),
            Event::Key(key(KeyCode::F(10))),
            // F10 now asks before quitting; Enter answers Yes.
            Event::Key(key(KeyCode::Enter)),
        ],
    )
}
