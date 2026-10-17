//! A `NetBox` panel through the real dispatcher: its keys are asked first,
//! F5/F6 from a file panel refuse it as the destination, and a menu row
//! that would act on the files hidden beneath it is refused as its key is.

use std::fs;

use tempfile::TempDir;

use super::*;
use crate::action::ToolCommand;
use crate::menu::{Entry, Heading, Menu, MenuCommand};
use crate::models::PanelSide;
use crate::sessions::SessionStore;
use crate::test_support::netbox::{app_in, app_on_sessions, open_empty_sessions, session};

fn terminal() -> Terminal<WritableTestBackend> {
    Terminal::new(WritableTestBackend::new(80, 24)).expect("test terminal")
}

/// A file under the cursor of the active left panel, the right panel showing
/// the sessions list.
fn beside_the_sessions(dir: &TempDir) -> App {
    fs::write(dir.path().join("a.txt"), "a").expect("write");
    let mut app = app_in(dir.path());
    let index = app
        .left_panel
        .entries
        .iter()
        .position(|entry| entry.name == "a.txt")
        .expect("listed");
    app.left_panel.select(Some(index));
    open_empty_sessions(&mut app, PanelSide::Right);
    app
}

#[test]
fn f5_and_f6_refuse_a_sessions_list_as_the_destination() {
    let dir = TempDir::new().expect("temp dir");
    let mut app = beside_the_sessions(&dir);
    let mut terminal = terminal();
    for code in [KeyCode::F(5), KeyCode::F(6)] {
        dispatch_key(&mut terminal, &mut app, key_event(code));
        assert!(!app.is_overlay_active(), "{code:?}: no copy prompt");
        assert_eq!(
            app.status_text(),
            "F5 and F6 need a folder in the other panel; it shows NetBox sessions or a host"
        );
    }
}

#[test]
fn f5_between_two_file_panels_still_asks_where_to() {
    let dir = TempDir::new().expect("temp dir");
    let mut app = beside_the_sessions(&dir);
    app.right_panel.kind = PanelKind::Files;
    dispatch_key(&mut terminal(), &mut app, key_event(KeyCode::F(5)));
    assert!(matches!(app.overlay, Overlay::CopyPrompt(_)));
}

#[test]
fn the_dispatcher_asks_the_sessions_list_before_the_file_actions() {
    let dir = TempDir::new().expect("temp dir");
    let mut store = SessionStore::default();
    store.upsert(session("a.test", None));
    let mut app = app_on_sessions(dir.path(), store);
    let mut terminal = terminal();
    dispatch_key(&mut terminal, &mut app, key_event(KeyCode::Down));
    dispatch_key(&mut terminal, &mut app, key_event(KeyCode::F(8)));
    assert!(
        matches!(app.overlay, Overlay::Question(_)),
        "the session's F8"
    );
    app.close_overlay();
    dispatch_key(&mut terminal, &mut app, key_event(KeyCode::Tab));
    assert_eq!(
        app.active_panel,
        PanelSide::Right,
        "Tab still switches panels"
    );
}

/// The row of `heading` that runs `action`.
fn row(heading: Heading, action: Action) -> Entry {
    Menu::open(heading)
        .entries()
        .iter()
        .copied()
        .find(|entry| entry.action() == Some(action))
        .expect("the menu has the row")
}

/// Choose `entry` as Enter or a click does, and let the dispatcher run it
/// once the drop-down has closed.
fn choose(app: &mut App, entry: Entry) {
    ops::menu::activate(app, entry);
    run_pending_row(&mut terminal(), app);
}

#[test]
fn a_row_for_the_files_is_refused_over_the_sessions_list() {
    let dir = TempDir::new().expect("temp dir");
    let mut app = app_on_sessions(dir.path(), SessionStore::default());
    choose(
        &mut app,
        row(Heading::Commands, Action::Menu(MenuCommand::Compare)),
    );
    assert_eq!(
        app.status_text(),
        "NetBox: this command works on files; press Enter on `..` to return to them"
    );
    assert!(!app.is_overlay_active());
    assert!(app.left_panel.marked.is_empty(), "nothing was compared");
}

#[test]
fn a_row_of_the_programs_own_still_runs_over_the_sessions_list() {
    let dir = TempDir::new().expect("temp dir");
    let mut app = app_on_sessions(dir.path(), SessionStore::default());
    let host = crate::test_support::processes::FixtureHost::new(Vec::new());
    app.task_host.install(std::sync::Arc::new(host));
    choose(
        &mut app,
        row(Heading::Commands, Action::Tool(ToolCommand::Tasks)),
    );
    assert!(app.is_overlay_active(), "the task list opened");
}

/// Registering the MCP server at All projects touches no entry of the
/// hidden file list, so its row runs over a `NetBox` panel too.
#[test]
fn the_install_mcp_server_row_runs_over_the_sessions_list() {
    let dir = TempDir::new().expect("temp dir");
    let mut app = app_on_sessions(dir.path(), SessionStore::default());
    choose(
        &mut app,
        row(Heading::Commands, Action::Menu(MenuCommand::McpInstall)),
    );
    let Overlay::McpInstall(dialog) = &app.overlay else {
        panic!(
            "the Install MCP server dialog opened: {}",
            app.status_text()
        );
    };
    assert_eq!(dialog.folder, None, "the hidden folder is not offered");
}

/// The Files rows whose keys `NetBox` gives a session act on the session,
/// as the keys do: F8 on the row asks before deleting it.
#[test]
fn a_files_row_the_sessions_list_owns_acts_on_the_session() {
    let dir = TempDir::new().expect("temp dir");
    let mut store = SessionStore::default();
    store.upsert(session("a.test", None));
    let mut app = app_on_sessions(dir.path(), store);
    dispatch_key(&mut terminal(), &mut app, key_event(KeyCode::Down));
    choose(&mut app, row(Heading::Files, Action::DeleteConfirm));
    assert!(
        matches!(app.overlay, Overlay::Question(_)),
        "the session's F8, not the file list's: {}",
        app.status_text()
    );
}

#[test]
fn over_a_file_list_a_row_for_the_files_runs() {
    let dir = TempDir::new().expect("temp dir");
    let mut app = app_in(dir.path());
    choose(
        &mut app,
        row(Heading::Commands, Action::Menu(MenuCommand::Compare)),
    );
    assert!(
        !app.status_text().starts_with("NetBox:"),
        "{}",
        app.status_text()
    );
}
