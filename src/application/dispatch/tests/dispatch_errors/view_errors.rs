//! Purpose: F3's failures — a binary file and an unreadable one — reach the status bar instead of ending the event loop.

use super::*;

/// F3 on a binary file is refused *inside* `handle_view` (it sets
/// `Status::BinaryFile` and returns `Ok`), so what this pins is the refusal
/// reaching the screen — NOT the dispatcher's error handling, which no longer
/// sees an error here at all. See
/// `test_view_io_error_is_reported_instead_of_killing_the_event_loop` for that.
#[test]
fn test_view_binary_file_shows_the_refusal_in_the_status_bar() -> io::Result<()> {
    let temp_dir = TempDir::new()?;
    fs::write(temp_dir.path().join("photo.bin"), [0xff, 0xd8, 0x00, 0x01])?;
    let app = app_at(temp_dir.path(), "photo.bin")?;

    let backend = WritableTestBackend::new(120, 24);
    let mut terminal = Terminal::new(backend)?;
    let mut events = vec![
        Event::Key(key_event(KeyCode::F(3))),
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

    assert!(
        result.is_ok(),
        "a binary file must not kill the file manager: {result:?}"
    );
    assert!(
        buffer_text(&terminal).contains("binary"),
        "the failure belongs in the status bar"
    );
    Ok(())
}

/// The same refusal, inspected on the `App`: status set, viewer never opened.
#[test]
fn test_view_binary_file_sets_binary_status_and_opens_no_viewer() -> io::Result<()> {
    let temp_dir = TempDir::new()?;
    fs::write(temp_dir.path().join("photo.bin"), [0xff, 0xd8, 0x00, 0x01])?;
    let mut app = app_at(temp_dir.path(), "photo.bin")?;

    dispatch(&mut app, Action::View)?;

    assert!(
        app.status_text().contains("Cannot view binary file"),
        "status was {:?}",
        app.status_text()
    );
    assert!(matches!(app.overlay, Overlay::None));
    Ok(())
}

/// A handler that genuinely returns `Err` through the F3 arm of the dispatcher.
///
/// The binary-file tests above cannot pin `report`: `handle_view` answers a
/// binary file with `Ok`, so `report(app, handle_view)` and a bare
/// `handle_view(app)?` behave identically and the arm ships unpinned. Here the
/// file is deleted after the panel listed it — an ordinary race with anything
/// else writing to the directory — so `fs::metadata` fails and the arm has a
/// real error to swallow: without `report` it either leaves the status bar empty
/// or (the pre-fix shape) propagates out of `run_app_with_event_reader` and
/// takes the whole file manager down with it.
#[test]
fn test_view_io_error_is_reported_instead_of_killing_the_event_loop() -> io::Result<()> {
    let temp_dir = TempDir::new()?;
    let vanished = temp_dir.path().join("gone.txt");
    fs::write(&vanished, "data")?;
    let app = app_at(temp_dir.path(), "gone.txt")?;
    fs::remove_file(&vanished)?;

    let backend = WritableTestBackend::new(120, 24);
    let mut terminal = Terminal::new(backend)?;
    let mut events = vec![
        Event::Key(key_event(KeyCode::F(3))),
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

    assert!(
        result.is_ok(),
        "a failed F3 must not kill the file manager: {result:?}"
    );
    // "Error:" is written by `report` and by nothing else — `Status::Error` is
    // the only variant with that prefix.
    assert!(
        buffer_text(&terminal).contains("Error:"),
        "the failure belongs in the status bar, not in the loop's return value"
    );
    Ok(())
}
