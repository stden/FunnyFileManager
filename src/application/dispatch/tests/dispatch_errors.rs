//! Unit tests: the main-view dispatcher reports handler failures in the status
//! bar instead of killing the event loop, and navigation drops stale marks.

use super::*;

use crate::models::{Panel, PanelSide};

use std::fs;

use tempfile::TempDir;

mod view_errors;

/// Run a single [`Action`] through the real dispatcher so the assertions can
/// inspect the `App` afterwards (the event loop consumes it).
fn dispatch(app: &mut App, action: Action) -> io::Result<()> {
    let backend = WritableTestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend)?;
    // `apply_action` returns no Result at all: a handler failure can only reach
    // the status bar, never the caller.
    apply_action(&mut terminal, app, action, true);
    Ok(())
}

fn app_at(dir: &Path, selected: &str) -> io::Result<App> {
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

/// The same rule on a second arm: F7 in a directory that no longer exists.
///
/// F7 opens a dialog now, so the failure happens when the name is confirmed
/// rather than when the key is pressed — the error still has to reach the
/// status bar instead of ending the session.
#[test]
fn test_mkdir_error_is_reported_not_fatal() -> io::Result<()> {
    use crossterm::event::{KeyEvent, KeyModifiers};

    let temp_dir = TempDir::new()?;
    let doomed = temp_dir.path().join("workdir");
    fs::create_dir(&doomed)?;
    let mut app = app_at(&doomed, "..")?;
    fs::remove_dir(&doomed)?;

    dispatch(&mut app, Action::Mkdir)?;
    for ch in "child".chars() {
        process_popup_input(
            &mut app,
            KeyEvent::new(KeyCode::Char(ch), KeyModifiers::empty()),
        );
    }
    process_popup_input(
        &mut app,
        KeyEvent::new(KeyCode::Enter, KeyModifiers::empty()),
    );

    assert!(
        !app.status_text().is_empty() && app.status_text() != "F1 Help",
        "the failure belongs in the status bar, status was {:?}",
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

/// F11 opens Far's Plugin commands list; its first row is the MCP status
/// screen in the read-only viewer, which F11 used to open outright.
#[test]
fn test_plugin_commands_action_opens_the_list_whose_first_row_is_mcp_status() -> io::Result<()> {
    let temp_dir = TempDir::new()?;
    fs::write(temp_dir.path().join("file.txt"), "data")?;
    let mut app = app_at(temp_dir.path(), "file.txt")?;

    dispatch(&mut app, Action::PluginCommands)?;
    assert!(matches!(app.overlay, Overlay::PluginCommands(_)));
    ops::plugin_commands::run(&mut app, models::PluginCommand::McpStatus)?;

    let viewer = app.get_viewer().expect("MCP status must open the viewer");
    assert_eq!(viewer.title(), "MCP Server Status");
    assert!(
        viewer.lines.iter().any(|line| line.starts_with("Server: ")),
        "the viewer must report where the MCP server is looked for: {:?}",
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
