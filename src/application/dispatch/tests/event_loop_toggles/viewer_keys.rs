//! Purpose: F3 opens the viewer overlay on the selected file.

use super::*;

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
fn test_run_app_f3_renders_viewer_overlay() -> io::Result<()> {
    let temp_dir = TempDir::new().expect("test setup");
    fs::write(temp_dir.path().join("view_me.txt"), b"hello\nworld").expect("test setup");

    let mut app = App::new().expect("test setup");
    app.left_panel = Panel::new(temp_dir.path().to_path_buf()).expect("test setup");
    app.right_panel = Panel::new(temp_dir.path().to_path_buf()).expect("test setup");

    assert!(app.left_panel.select_named("view_me.txt"));

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
    // Far's status line: the full path on the left, the fields on the right.
    // "hello\nworld" is 11 bytes, and both lines fit, so it is at 100%.
    let path = temp_dir.path().join("view_me.txt").display().to_string();
    assert!(screen.contains(&path), "{screen}");
    assert!(
        screen.contains("│t│UTF-8│11      │Col 0   │100% │"),
        "{screen}"
    );
    assert!(screen.contains("hello"), "{screen}");
    assert!(screen.contains("world"), "{screen}");
    Ok(())
}
