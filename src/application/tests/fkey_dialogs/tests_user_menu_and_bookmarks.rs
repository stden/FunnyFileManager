//! Purpose: plain F2 opens and closes the user menu, and bookmarks round-trip through the `:` command line.

use super::*;
use crate::models::BookmarkSlot;

// --- Bookmarks live on the `:` command line, not on an overlay ---

/// Plain F2 opens the existing Commands drop-down as the user menu. Esc closes
/// it cleanly so the next F10 reaches the main view.
#[test]
fn test_f2_opens_and_closes_the_user_menu() -> io::Result<()> {
    let temp_dir = TempDir::new().expect("test setup");
    let mut app = App::new().expect("test setup");
    app.left_panel = Panel::new(temp_dir.path().to_path_buf()).expect("test setup");
    app.right_panel = Panel::new(temp_dir.path().to_path_buf()).expect("test setup");

    let backend = WritableTestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).expect("test setup");

    let mut events = vec![
        Event::Key(key_event(KeyCode::F(2))),
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

    assert!(result.is_ok(), "F2's menu must close and return control");
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

    let mut events = command_events("bookmark set 3");

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

fn command_events(command: &str) -> impl Iterator<Item = Event> + '_ {
    std::iter::once(KeyCode::Char(':'))
        .chain(command.chars().map(KeyCode::Char))
        .chain([KeyCode::Enter, KeyCode::F(10), KeyCode::Enter])
        .map(|code| Event::Key(key_event(code)))
}
