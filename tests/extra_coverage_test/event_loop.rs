//! Purpose: A full keyboard workflow through the event loop.

use super::*;

#[test]
fn test_full_event_loop_keys() -> io::Result<()> {
    let temp_dir = TempDir::new()?;
    File::create(temp_dir.path().join("file1.txt"))?;
    File::create(temp_dir.path().join("file2.txt"))?;

    let mut app = App::new()?;
    app.left_panel = Panel::new(temp_dir.path().to_path_buf())?;
    app.right_panel = Panel::new(temp_dir.path().to_path_buf())?;

    let mut terminal =
        ratatui::Terminal::new(funny_file_manager::WritableTestBackend::new(80, 24))?;

    let mut events = workflow_keys();

    funny_file_manager::run_app_with_event_reader(&mut terminal, app, || {
        events
            .next()
            .ok_or_else(|| io::Error::other("no more events"))
    })?;

    Ok(())
}

fn workflow_keys() -> std::vec::IntoIter<Event> {
    vec![
        // Command line
        Event::Key(key_event(KeyCode::Char(':'))),
        Event::Key(key_event(KeyCode::Char('p'))),
        Event::Key(key_event(KeyCode::Char('w'))),
        Event::Key(key_event(KeyCode::Char('d'))),
        Event::Key(key_event(KeyCode::Enter)),
        // Marking with Space
        Event::Key(key_event(KeyCode::Down)),
        Event::Key(key_event(KeyCode::Char(' '))),
        // Marking with Insert
        Event::Key(key_event(KeyCode::Insert)),
        // Navigation
        Event::Key(key_event(KeyCode::Home)),
        Event::Key(key_event(KeyCode::End)),
        // Ctrl+R Refresh
        Event::Key(KeyEvent::new(KeyCode::Char('r'), KeyModifiers::CONTROL)),
        // Ctrl+U Swap
        Event::Key(KeyEvent::new(KeyCode::Char('u'), KeyModifiers::CONTROL)),
        // Ctrl+O Console
        Event::Key(KeyEvent::new(KeyCode::Char('o'), KeyModifiers::CONTROL)),
        // Ctrl+\ Root
        Event::Key(KeyEvent::new(KeyCode::Char('\\'), KeyModifiers::CONTROL)),
        // Shift+F4 New File
        Event::Key(KeyEvent::new(KeyCode::F(4), KeyModifiers::SHIFT)),
        Event::Key(key_event(KeyCode::Esc)),
        // Alt+F1 Drives
        Event::Key(KeyEvent::new(KeyCode::F(1), KeyModifiers::ALT)),
        Event::Key(key_event(KeyCode::Esc)),
        // F1 Help
        Event::Key(key_event(KeyCode::F(1))),
        Event::Key(key_event(KeyCode::F(1))),
        // Quit
        Event::Key(key_event(KeyCode::F(10))),
        // F10 now asks before quitting; Enter answers Yes.
        Event::Key(key_event(KeyCode::Enter)),
    ]
    .into_iter()
}
