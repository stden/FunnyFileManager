//! Unit tests: over a `NetBox` panel, a menu row that would act on the files
//! hidden beneath is refused, as its key would be, and the program's rows
//! still run.

use tempfile::TempDir;

use super::activate;
use crate::menu::{Entry, Heading, Menu, MenuCommand};
use crate::sessions::SessionStore;
use crate::test_support::netbox::app_on_sessions;

/// The row of `heading` that runs `command`.
fn row(heading: Heading, command: MenuCommand) -> Entry {
    Menu::open(heading)
        .entries()
        .iter()
        .copied()
        .find(|entry| entry.menu_command() == Some(command))
        .expect("the menu has the row")
}

#[test]
fn a_row_for_the_files_is_refused_over_the_sessions_list() {
    let dir = TempDir::new().expect("temp dir");
    let mut app = app_on_sessions(dir.path(), SessionStore::default());
    activate(&mut app, row(Heading::Commands, MenuCommand::Compare));
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
    activate(&mut app, row(Heading::Commands, MenuCommand::Tasks));
    assert!(app.is_overlay_active(), "the task list opened");
}

#[test]
fn over_a_file_list_a_row_for_the_files_runs() {
    let dir = TempDir::new().expect("temp dir");
    let mut app = crate::test_support::netbox::app_in(dir.path());
    activate(&mut app, row(Heading::Commands, MenuCommand::Compare));
    assert!(
        !app.status_text().starts_with("NetBox:"),
        "{}",
        app.status_text()
    );
}
