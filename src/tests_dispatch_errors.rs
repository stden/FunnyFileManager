//! Unit tests: the main-view dispatcher reports handler failures in the status
//! bar instead of killing the event loop, and navigation drops stale marks.
#![allow(unused_imports)]

use super::tests::{buffer_text, key_event};
use super::*;
use crate::status::Status;

use crate::models::{Panel, PanelSide};

use crossterm::event::KeyCode;

use std::fs;

use tempfile::TempDir;

/// Run a single [`Action`] through the real dispatcher so the assertions can
/// inspect the `App` afterwards (the event loop consumes it).
fn dispatch(app: &mut App, action: Action) -> io::Result<()> {
    let backend = WritableTestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend)?;
    let mut last_nav_time = Instant::now().checked_sub(Duration::from_secs(1)).unwrap();
    // `apply_action` returns no Result at all: a handler failure can only reach
    // the status bar, never the caller.
    apply_action(
        &mut terminal,
        app,
        action,
        &mut last_nav_time,
        Duration::from_millis(0),
    );
    Ok(())
}

fn app_at(dir: &std::path::Path, selected: &str) -> io::Result<App> {
    let mut app = App::new()?;
    app.left_panel = Panel::new(dir.to_path_buf())?;
    app.right_panel = Panel::new(dir.to_path_buf())?;
    app.active_panel = PanelSide::Left;
    let index = app
        .left_panel
        .entries
        .iter()
        .position(|entry| entry.name == selected)
        .expect("test setup: entry must be listed");
    app.left_panel.select(Some(index));
    Ok(app)
}

// -----------------------------------------------------------------------
// Handler errors must not be fatal
// -----------------------------------------------------------------------

/// F3 on a binary file is refused *inside* `handle_view` (it sets
/// `Status::BinaryFile` and returns `Ok`), so what this pins is the refusal
/// reaching the screen — NOT the dispatcher's error handling, which no longer
/// sees an error here at all. See
/// `test_view_io_error_is_reported_instead_of_killing_the_event_loop` for that.
#[test]
fn test_view_binary_file_shows_the_refusal_in_the_status_bar() -> io::Result<()> {
    let temp_dir = TempDir::new()?;
    fs::write(temp_dir.path().join("photo.bin"), [0xff, 0xfe, 0x00, 0x01])?;
    let app = app_at(temp_dir.path(), "photo.bin")?;

    let backend = WritableTestBackend::new(120, 24);
    let mut terminal = Terminal::new(backend)?;
    let mut events = vec![
        Event::Key(key_event(KeyCode::F(3))),
        Event::Key(key_event(KeyCode::F(10))),
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
    fs::write(temp_dir.path().join("photo.bin"), [0xff, 0xfe, 0x00, 0x01])?;
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

/// The same rule on a second arm: F7 in a directory that no longer exists.
#[test]
fn test_mkdir_error_is_reported_not_fatal() -> io::Result<()> {
    let temp_dir = TempDir::new()?;
    let doomed = temp_dir.path().join("workdir");
    fs::create_dir(&doomed)?;
    let mut app = app_at(&doomed, "..")?;
    fs::remove_dir(&doomed)?;

    dispatch(&mut app, Action::Mkdir)?;

    assert!(
        app.status_text().starts_with("Error:"),
        "status was {:?}",
        app.status_text()
    );
    Ok(())
}

/// Enter on a directory that vanished after the listing was taken.
#[test]
fn test_open_missing_dir_is_reported_not_fatal() -> io::Result<()> {
    let temp_dir = TempDir::new()?;
    let gone = temp_dir.path().join("gone");
    fs::create_dir(&gone)?;
    let mut app = app_at(temp_dir.path(), "gone")?;
    fs::remove_dir(&gone)?;

    dispatch(&mut app, Action::Open)?;

    assert_eq!(app.left_panel.path, temp_dir.path());
    assert!(
        app.status_text().starts_with("Error:"),
        "status was {:?}",
        app.status_text()
    );
    Ok(())
}

/// Alt+F4 is wired to the *external* editor: a directory is refused before the
/// terminal is ever suspended, and no built-in editor overlay is opened (which
/// is what distinguishes this arm from plain F4).
#[test]
fn test_edit_external_on_a_directory_is_refused_without_opening_the_editor() -> io::Result<()> {
    let temp_dir = TempDir::new()?;
    fs::create_dir(temp_dir.path().join("notes"))?;
    let mut app = app_at(temp_dir.path(), "notes")?;

    dispatch(&mut app, Action::EditExternal)?;

    assert_eq!(app.status_text(), "Editing directories is not supported");
    assert!(
        matches!(app.overlay, Overlay::None),
        "no overlay may open for the external editor"
    );
    Ok(())
}

/// F9 opens the MCP status screen in the read-only viewer.
#[test]
fn test_mcp_status_action_opens_the_mcp_viewer() -> io::Result<()> {
    let temp_dir = TempDir::new()?;
    fs::write(temp_dir.path().join("file.txt"), "data")?;
    let mut app = app_at(temp_dir.path(), "file.txt")?;

    dispatch(&mut app, Action::McpStatus)?;

    let viewer = app.get_viewer().expect("MCP status must open the viewer");
    assert_eq!(viewer.title, "MCP Server Status");
    assert!(
        viewer.lines.iter().any(|line| line.starts_with("Binary: ")),
        "the viewer must report where the MCP binary is looked for: {:?}",
        viewer.lines
    );
    assert_eq!(app.status_text(), "MCP Server — press Esc to close");
    Ok(())
}

// -----------------------------------------------------------------------
// Ctrl+\ must not carry marks to the root
// -----------------------------------------------------------------------

/// Marks are bare names, so keeping them across a path change re-binds them to
/// whatever happens to share the name in the new directory (`/tmp`, `/bin`, ...).
#[test]
fn test_go_root_clears_marks() -> io::Result<()> {
    let temp_dir = TempDir::new()?;
    fs::write(temp_dir.path().join("tmp"), "not the real /tmp")?;
    let mut app = app_at(temp_dir.path(), "tmp")?;
    assert!(app.left_panel.toggle_mark_selected());
    assert!(app.left_panel.marked.contains("tmp"));

    dispatch(&mut app, Action::GoRoot)?;

    assert_eq!(
        app.left_panel.path,
        PathBuf::from(if cfg!(windows) { "C:\\" } else { "/" })
    );
    assert!(
        app.left_panel.marked.is_empty(),
        "marks must not survive the jump to root: {:?}",
        app.left_panel.marked
    );
    Ok(())
}

// -----------------------------------------------------------------------
// Logging is opt-in, payload-free and owner-only
// -----------------------------------------------------------------------

#[test]
fn test_key_label_hides_typed_characters() {
    assert_eq!(key_label(KeyCode::Char('s')), "Char");
    assert_eq!(key_label(KeyCode::Char('3')), "Char");
    assert_eq!(key_label(KeyCode::F(3)), "F(3)");
    assert_eq!(key_label(KeyCode::Enter), "Enter");
}

/// The same rule as `key_label`, one level up: actions are logged before an
/// overlay consumes them, so an action carrying a typed character must not
/// print it. A derived `Debug` would write every password typed at the `:`
/// prompt into the log file, one keystroke per line.
#[test]
fn test_action_label_hides_typed_characters() {
    assert_eq!(
        action_label(Action::CommandLineChar('s')),
        "CommandLineChar(..)"
    );
    assert_eq!(action_label(Action::QuickSearch('s')), "QuickSearch(..)");
    assert_eq!(action_label(Action::Quit), "Quit");
    assert_eq!(action_label(Action::Copy), "Copy");
}

#[test]
fn test_logging_is_off_unless_opted_in() {
    if std::env::var_os("FFM_LOG").is_none() {
        assert!(
            log_dir().is_none(),
            "logging must stay off until FFM_LOG=1 is set"
        );
    }
}

#[test]
fn test_append_log_line_is_owner_only() -> io::Result<()> {
    let temp_dir = TempDir::new()?;
    let path = temp_dir.path().join("ffm_actions.log");

    append_log_line(&path, "hello");
    append_log_line(&path, "again");

    let contents = fs::read_to_string(&path)?;
    assert!(contents.contains("] hello"), "{contents}");
    assert!(contents.contains("] again"), "{contents}");

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mode = fs::metadata(&path)?.permissions().mode() & 0o777;
        assert_eq!(mode, 0o600, "log must not be readable by other users");
    }
    Ok(())
}

/// A log that cannot be opened is skipped silently — logging must never break
/// the file manager.
#[test]
fn test_append_log_line_ignores_open_failure() -> io::Result<()> {
    let temp_dir = TempDir::new()?;
    let blocked = temp_dir.path().join("ffm_actions.log");
    fs::create_dir(&blocked)?;

    append_log_line(&blocked, "swallowed");

    assert!(blocked.is_dir());
    Ok(())
}
