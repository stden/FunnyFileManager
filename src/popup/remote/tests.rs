//! Unit tests: the remote panel's keys move and enter, refuse what would act
//! on the hidden files, and hand the program's keys back.

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use tempfile::TempDir;

use super::{apply, process, NOT_FILES};
use crate::action::netbox::{resolve, NetboxCommand, RowCommand};
use crate::models::{App, ListMotion, PanelKind, RemotePanel};
use crate::test_support::chord;
use crate::test_support::netbox::{app_in, app_on_host};

const FILES: [(&str, u64); 2] = [("/etc/hostname", 5), ("/var/www/index.html", 10)];

/// The keys the sessions list gives a session, and the one that makes one.
const SESSION_KEYS: [(KeyCode, KeyModifiers, NetboxCommand); 5] = [
    (
        KeyCode::F(3),
        KeyModifiers::NONE,
        NetboxCommand::Row(RowCommand::ViewSession),
    ),
    (
        KeyCode::F(4),
        KeyModifiers::NONE,
        NetboxCommand::Row(RowCommand::EditSession),
    ),
    (
        KeyCode::F(4),
        KeyModifiers::SHIFT,
        NetboxCommand::NewSession,
    ),
    (
        KeyCode::F(5),
        KeyModifiers::NONE,
        NetboxCommand::Row(RowCommand::CopySession),
    ),
    (
        KeyCode::F(8),
        KeyModifiers::NONE,
        NetboxCommand::Row(RowCommand::DeleteSession),
    ),
];

fn bare(code: KeyCode) -> KeyEvent {
    KeyEvent::new(code, KeyModifiers::NONE)
}

fn remote(app: &App) -> &RemotePanel {
    let PanelKind::Remote(panel) = &app.left_panel.kind else {
        panic!("the remote panel");
    };
    panel
}

#[test]
fn the_list_keys_move_and_enter_opens_a_directory() {
    let dir = TempDir::new().expect("temp dir");
    let mut app = app_on_host(dir.path(), &FILES, None);
    assert!(process(&mut app, bare(KeyCode::End)));
    assert_eq!(remote(&app).cursor(), 2);
    assert!(process(&mut app, bare(KeyCode::Up)));
    assert!(process(&mut app, bare(KeyCode::Enter)));
    crate::test_support::finish_remote(&mut app);
    assert_eq!(remote(&app).path().as_str(), "/etc");
}

#[test]
fn a_file_key_is_refused_and_a_program_key_passes() {
    let dir = TempDir::new().expect("temp dir");
    let mut app = app_on_host(dir.path(), &FILES, None);
    for refused in [KeyCode::F(3), KeyCode::F(5), KeyCode::F(8)] {
        assert!(process(&mut app, bare(refused)), "{refused:?}");
        assert!(!app.is_overlay_active(), "{refused:?}");
    }
    assert_eq!(
        remote(&app).cursor(),
        0,
        "parent is not a file operation target"
    );
    assert!(!process(&mut app, bare(KeyCode::Tab)));
    assert!(!process(
        &mut app,
        KeyEvent::new(KeyCode::F(2), KeyModifiers::ALT)
    ));
}

/// A host has no session under its cursor and no list to add one to: the
/// five session commands are handed back, and their keys are refused for the
/// files they would reach — Shift+F4 would make a file beneath the panel.
#[test]
fn remote_file_commands_are_handled_without_reaching_hidden_local_files() {
    let dir = TempDir::new().expect("temp dir");
    for (code, modifiers, command) in SESSION_KEYS {
        let key = KeyEvent::new(code, modifiers);
        let mut app = app_on_host(dir.path(), &FILES, None);
        let resolved = resolve(&app.keymap.netbox_panel, key);
        assert_eq!(resolved, Some(command), "{key:?}");
        assert_eq!(
            apply(&mut app, command),
            command != NetboxCommand::NewSession,
            "{command:?}"
        );
        assert_ne!(app.status_text(), NOT_FILES, "{command:?}");
        assert!(process(&mut app, key), "{key:?}");
        if command == NetboxCommand::NewSession {
            assert_eq!(app.status_text(), NOT_FILES, "{key:?}");
        }
        assert!(!app.is_overlay_active(), "{key:?}");
    }
}

/// The panel reads the keys of the app's keymap: a key given to a move
/// moves, and a session command's new key is handed back like its old one.
#[test]
fn the_panel_obeys_the_keys_of_the_apps_keymap() {
    let dir = TempDir::new().expect("temp dir");
    let mut app = app_on_host(dir.path(), &FILES, None);
    let keys = &mut app.keymap.netbox_panel;
    keys.bind(NetboxCommand::Move(ListMotion::End), chord("Ctrl+F3"));
    keys.bind(NetboxCommand::Row(RowCommand::ViewSession), chord("Tab"));

    assert!(process(&mut app, chord("Ctrl+F3").to_event()));
    assert_eq!(remote(&app).cursor(), 2);
    assert!(
        process(&mut app, bare(KeyCode::Tab)),
        "remapped remote view command"
    );
    assert!(!app.is_overlay_active());
}

#[test]
fn another_kind_of_panel_is_not_the_remote_panels() {
    let dir = TempDir::new().expect("temp dir");
    let mut app = app_in(dir.path());
    assert!(!process(&mut app, bare(KeyCode::Enter)));
}
