//! Unit tests for the shared `.kdbx` open trigger.
//!
//! The module exists so `Enter` and `F3` cannot drift apart; the tests
//! therefore pin the *shared* rules — what counts as a vault file, and that
//! the directory guard wins — and drive the `Enter` cases through the real
//! dispatch, so a regression in the `Action::Open` wiring cannot hide behind
//! a helper-only test.

use std::fs::{self, File};
use std::path::Path;

use crossterm::event::KeyCode;
use ratatui::Terminal;
use tempfile::TempDir;

use super::{is_vault_file, open_vault_if_kdbx};
use crate::models::{App, Overlay, Panel};

/// An `App` whose active (left) panel lists `dir`.
fn app_showing(dir: &TempDir) -> App {
    let mut app = App::new().expect("test app");
    app.left_panel = Panel::new(dir.path().to_path_buf()).expect("panel");
    app
}

/// Put the cursor on the entry named `name`, wherever the sort left it.
fn select(app: &mut App, name: &str) {
    let index = app
        .left_panel
        .entries
        .iter()
        .position(|entry| entry.name == name)
        .unwrap_or_else(|| panic!("no entry named {name}"));
    app.left_panel.select(Some(index));
}

/// Send `Enter` through the real dispatcher, so the test exercises the
/// `Action::Open` arm and not a private helper the arm may have drifted from.
fn press_enter(app: &mut App) {
    let mut terminal =
        Terminal::new(crate::WritableTestBackend::new(80, 24)).expect("test terminal");
    crate::application::dispatch_key(
        &mut terminal,
        app,
        crate::test_support::key_event(KeyCode::Enter),
    );
}

#[test]
fn vault_extensions_match_case_insensitively_including_the_typo() {
    for name in ["vault.kdbx", "VAULT.KDBX", "vault.KbDx", "vault.kbdx"] {
        assert!(
            is_vault_file(Path::new(name)),
            "{name} should open the vault browser"
        );
    }
    // `.kdbx` is a hidden file with no extension, and `vault.kdbx.bak` is a
    // backup — both must keep their ordinary open/view behaviour.
    for name in ["vault.txt", "vault", ".kdbx", "vault.kdbx.bak"] {
        assert!(
            !is_vault_file(Path::new(name)),
            "{name} must not open the vault browser"
        );
    }
}

#[test]
fn a_directory_named_like_a_vault_is_not_hijacked() {
    let dir = TempDir::new().expect("temp dir");
    fs::create_dir(dir.path().join("secrets.kdbx")).expect("mkdir");
    let mut app = app_showing(&dir);
    select(&mut app, "secrets.kdbx");

    assert!(!open_vault_if_kdbx(&mut app));
    assert!(matches!(app.overlay, Overlay::None));
}

#[test]
fn opening_a_vault_file_raises_the_overlay_locked() {
    let dir = TempDir::new().expect("temp dir");
    File::create(dir.path().join("vault.kdbx")).expect("create vault");
    let mut app = app_showing(&dir);
    select(&mut app, "vault.kdbx");

    assert!(open_vault_if_kdbx(&mut app));

    let session = app.kdbx_session().expect("vault overlay is open");
    assert!(
        session.state.is_locked(),
        "a fresh session must ask for the master password"
    );
    assert!(
        app.status_text().contains("enter the master password"),
        "{}",
        app.status_text()
    );
}

#[test]
fn enter_opens_both_vault_extensions() {
    for name in ["vault.kdbx", "vault.kbdx"] {
        let dir = TempDir::new().expect("temp dir");
        File::create(dir.path().join(name)).expect("create vault");
        let mut app = app_showing(&dir);
        select(&mut app, name);
        let before = app.left_panel.path.clone();

        press_enter(&mut app);

        assert!(app.kdbx_session().is_some(), "Enter must open {name}");
        assert_eq!(
            app.left_panel.path, before,
            "the panel must not also navigate"
        );
    }
}

#[test]
fn enter_still_descends_into_directories() {
    let dir = TempDir::new().expect("temp dir");
    fs::create_dir(dir.path().join("plain")).expect("mkdir");
    let mut app = app_showing(&dir);
    select(&mut app, "plain");

    press_enter(&mut app);

    assert!(matches!(app.overlay, Overlay::None));
    assert_eq!(app.left_panel.path, dir.path().join("plain"));
}

/// A `.kdbx` on a FAT mount arrives with the executable bit set, and the
/// `Action::Open` arm runs executables. The vault check must win, or Enter
/// hands the database to a shell instead of opening it.
#[cfg(unix)]
#[test]
fn enter_prefers_vault_over_executable_flag() {
    use std::os::unix::fs::PermissionsExt;

    let dir = TempDir::new().expect("temp dir");
    let path = dir.path().join("vault.kdbx");
    fs::write(&path, "#!/bin/sh\necho ran-the-vault\n").expect("create vault");
    fs::set_permissions(&path, fs::Permissions::from_mode(0o755)).expect("chmod");
    let mut app = app_showing(&dir);
    select(&mut app, "vault.kdbx");

    press_enter(&mut app);

    assert!(
        app.kdbx_session().is_some(),
        "the vault browser must open, not the program"
    );
    assert_eq!(
        app.console.visible(usize::MAX).len(),
        0,
        "nothing may have been executed"
    );
}

#[test]
fn f3_views_a_vault_through_the_same_trigger_as_enter() {
    let dir = TempDir::new().expect("temp dir");
    File::create(dir.path().join("vault.kdbx")).expect("create vault");
    let mut app = app_showing(&dir);
    select(&mut app, "vault.kdbx");

    crate::ops::handle_view(&mut app).expect("view");

    assert!(
        app.kdbx_session().is_some(),
        "F3 must open the vault browser, not the plain viewer"
    );
}
