//! Unit tests: event loop misc (split out to keep files under 500 lines).
#![allow(unused_imports)]

use crate::quicksearch::QuickSearch;

use super::*;

use crate::models::Panel;

use crossterm::event::{KeyCode, KeyModifiers};

use std::fs;

use std::path::PathBuf;

use tempfile::TempDir;

#[cfg(test)]
mod tests_cursor_navigation;
#[cfg(test)]
mod tests_panel_selection;
#[cfg(test)]
mod tests_quick_search_keys;

// -----------------------------------------------------------------------
// Event loop: F10 quit
// -----------------------------------------------------------------------

#[test]
fn test_event_loop_f10_quit() -> io::Result<()> {
    let temp_dir = TempDir::new()?;
    let mut app = App::new()?;
    app.left_panel = Panel::new(temp_dir.path().to_path_buf())?;
    app.right_panel = Panel::new(temp_dir.path().to_path_buf())?;

    let backend = WritableTestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend)?;

    let mut events = vec![
        Event::Key(key_event(KeyCode::F(10))),
        // F10 now asks before quitting; Enter answers Yes.
        Event::Key(key_event(KeyCode::Enter)),
    ]
    .into_iter();

    let result = run_app_with_event_reader(&mut terminal, app, || {
        events
            .next()
            .ok_or_else(|| io::Error::new(io::ErrorKind::UnexpectedEof, "done"))
    });
    assert!(result.is_ok());
    Ok(())
}

// -----------------------------------------------------------------------
// Event loop: non-key event (resize) is ignored
// -----------------------------------------------------------------------

#[test]
fn test_event_loop_resize_event_ignored() -> io::Result<()> {
    let temp_dir = TempDir::new()?;
    let mut app = App::new()?;
    app.left_panel = Panel::new(temp_dir.path().to_path_buf())?;
    app.right_panel = Panel::new(temp_dir.path().to_path_buf())?;

    let backend = WritableTestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend)?;

    let mut events = vec![
        Event::Resize(100, 50),
        Event::Key(key_event(KeyCode::F(10))),
        // F10 now asks before quitting; Enter answers Yes.
        Event::Key(key_event(KeyCode::Enter)),
    ]
    .into_iter();

    let result = run_app_with_event_reader(&mut terminal, app, || {
        events
            .next()
            .ok_or_else(|| io::Error::new(io::ErrorKind::UnexpectedEof, "done"))
    });
    assert!(result.is_ok());
    Ok(())
}

// -----------------------------------------------------------------------
// Event loop: Ctrl+R refresh error path
// -----------------------------------------------------------------------

#[test]
fn test_event_loop_ctrl_r_refresh_error() -> io::Result<()> {
    let temp_dir = TempDir::new()?;
    let mut app = App::new()?;
    app.left_panel = Panel::new(temp_dir.path().to_path_buf())?;
    app.right_panel = Panel::new(temp_dir.path().to_path_buf())?;
    // Point panel to a nonexistent path to trigger refresh error
    app.left_panel.path = PathBuf::from("/nonexistent_refresh_path_99999");

    let backend = WritableTestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend)?;

    let mut events = vec![
        Event::Key(key_event_with_mods(
            KeyCode::Char('r'),
            KeyModifiers::CONTROL,
        )),
        Event::Key(key_event(KeyCode::F(10))),
        // F10 now asks before quitting; Enter answers Yes.
        Event::Key(key_event(KeyCode::Enter)),
    ]
    .into_iter();

    let result = run_app_with_event_reader(&mut terminal, app, || {
        events
            .next()
            .ok_or_else(|| io::Error::new(io::ErrorKind::UnexpectedEof, "done"))
    });
    assert!(result.is_ok());
    Ok(())
}

#[cfg(unix)]
#[test]
fn make_fifo_reports_the_tool_failure_for_an_occupied_path() {
    let temp = tempfile::TempDir::new().expect("temp dir");
    let path = temp.path().join("pipe");
    crate::test_support::make_fifo(&path).expect("first fifo");

    let error = crate::test_support::make_fifo(&path).expect_err("path is taken");

    assert!(error.to_string().contains("mkfifo"), "{error}");
}
