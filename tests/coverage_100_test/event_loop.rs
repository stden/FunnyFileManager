//! Purpose: The writable test backend, the event reader, debounce and the root hotkey.

use super::*;

#[test]
fn test_writable_backend_full_coverage() -> io::Result<()> {
    use funny_file_manager::WritableTestBackend;
    use ratatui::backend::Backend;
    use std::io::Write;

    let mut backend = WritableTestBackend::new(80, 24);

    // Test write method
    let bytes = backend.write(b"test")?;
    assert_eq!(bytes, 4);

    // Test flush (Write trait)
    Write::flush(&mut backend)?;

    // Test backend methods
    backend.hide_cursor()?;
    backend.show_cursor()?;
    backend.get_cursor_position()?;
    backend.set_cursor_position((10, 20))?;
    backend.clear()?;
    backend.size()?;
    backend.window_size()?;
    Backend::flush(&mut backend)?;

    Ok(())
}

#[test]
fn test_event_reader_coverage() -> io::Result<()> {
    use funny_file_manager::run_app_with_event_reader;
    use ratatui::Terminal;

    let temp_dir = TempDir::new()?;
    let mut app = App::new()?;
    app.left_panel = Panel::new(temp_dir.path().to_path_buf())?;
    app.right_panel = Panel::new(temp_dir.path().to_path_buf())?;

    let backend = funny_file_manager::WritableTestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend)?;

    // Test with event reader that returns non-key events
    let mut events = vec![
        Event::Resize(100, 50),
        Event::Key(key_event(KeyCode::F(10), KeyModifiers::empty())),
        // F10 now asks before quitting; Enter answers Yes.
        Event::Key(key_event(KeyCode::Enter, KeyModifiers::empty())),
    ]
    .into_iter();

    run_app_with_event_reader(&mut terminal, app, || {
        events
            .next()
            .ok_or_else(|| io::Error::new(io::ErrorKind::UnexpectedEof, "No more events"))
    })?;

    Ok(())
}

#[test]
#[cfg(coverage)]
fn test_coverage_specific_functions() -> io::Result<()> {
    // Test coverage-specific functions
    funny_file_manager::coverage_touch_logs();
    funny_file_manager::coverage_touch_backend()?;
    Ok(())
}

#[test]
fn test_navigation_with_debounce() -> io::Result<()> {
    use funny_file_manager::run_app_with_event_reader;
    use ratatui::Terminal;

    let temp_dir = TempDir::new()?;
    fs::write(temp_dir.path().join("file1.txt"), b"content1")?;
    fs::write(temp_dir.path().join("file2.txt"), b"content2")?;
    fs::write(temp_dir.path().join("file3.txt"), b"content3")?;

    let mut app = App::new()?;
    app.left_panel = Panel::new(temp_dir.path().to_path_buf())?;
    app.right_panel = Panel::new(temp_dir.path().to_path_buf())?;

    let backend = funny_file_manager::WritableTestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend)?;

    let mut events = vec![
        // Rapid navigation: Repeat events are ignored, Press events all land.
        Event::Key(key_event(KeyCode::Down, KeyModifiers::empty())),
        Event::Key(key_event(KeyCode::Down, KeyModifiers::empty())),
        Event::Key(key_event(KeyCode::Down, KeyModifiers::empty())),
        Event::Key(key_event(KeyCode::Down, KeyModifiers::empty())),
        Event::Key(key_event(KeyCode::Up, KeyModifiers::empty())),
        Event::Key(key_event(KeyCode::Up, KeyModifiers::empty())),
        Event::Key(key_event(KeyCode::Up, KeyModifiers::empty())),
        Event::Key(key_event(KeyCode::F(10), KeyModifiers::empty())),
        // F10 now asks before quitting; Enter answers Yes.
        Event::Key(key_event(KeyCode::Enter, KeyModifiers::empty())),
    ]
    .into_iter();

    run_app_with_event_reader(&mut terminal, app, || {
        events
            .next()
            .ok_or_else(|| io::Error::new(io::ErrorKind::UnexpectedEof, "No more events"))
    })?;

    Ok(())
}

#[test]
fn test_ctrl_backslash_unix_root() -> io::Result<()> {
    use funny_file_manager::run_app_with_event_reader;
    use ratatui::Terminal;

    let temp_dir = TempDir::new()?;
    let mut app = App::new()?;
    app.left_panel = Panel::new(temp_dir.path().to_path_buf())?;
    app.right_panel = Panel::new(temp_dir.path().to_path_buf())?;

    let backend = funny_file_manager::WritableTestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend)?;

    let mut events = vec![
        Event::Key(key_event(KeyCode::Char('\\'), KeyModifiers::CONTROL)),
        Event::Key(key_event(KeyCode::F(10), KeyModifiers::empty())),
        // F10 now asks before quitting; Enter answers Yes.
        Event::Key(key_event(KeyCode::Enter, KeyModifiers::empty())),
    ]
    .into_iter();

    run_app_with_event_reader(&mut terminal, app, || {
        events
            .next()
            .ok_or_else(|| io::Error::new(io::ErrorKind::UnexpectedEof, "No more events"))
    })?;

    Ok(())
}
