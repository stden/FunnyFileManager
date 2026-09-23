//! Unit tests: the remote panel's keys move and enter, refuse what would act
//! on the hidden files, and hand the program's keys back.

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use tempfile::TempDir;

use super::process;
use crate::models::{App, PanelKind, RemotePanel};
use crate::test_support::netbox::{app_in, app_on_host};

const FILES: [(&str, u64); 2] = [("/etc/hostname", 5), ("/var/www/index.html", 10)];

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
        app.status_text(),
        "NetBox: this key works on local files; Enter on `..` at the host's root leaves it"
    );
    assert!(!process(&mut app, bare(KeyCode::Tab)));
    assert!(!process(
        &mut app,
        KeyEvent::new(KeyCode::F(2), KeyModifiers::ALT)
    ));
}

#[test]
fn another_kind_of_panel_is_not_the_remote_panels() {
    let dir = TempDir::new().expect("temp dir");
    let mut app = app_in(dir.path());
    assert!(!process(&mut app, bare(KeyCode::Enter)));
}
