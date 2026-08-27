//! Unit tests for the shared `.kdbx` open trigger.
//!
//! The module exists so `Enter` and `F3` cannot drift apart; the tests
//! therefore pin the *shared* rules — what counts as a vault file, and that
//! the directory guard wins — rather than either caller's key handling.

use std::fs::{self, File};
use std::path::Path;

use tempfile::TempDir;

use super::{handle_open, is_vault_file, open_vault_if_kdbx};
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
    let vault_dir = dir.path().join("secrets.kdbx");
    fs::create_dir(&vault_dir).expect("mkdir");
    let mut app = app_showing(&dir);

    assert!(!open_vault_if_kdbx(&mut app, &vault_dir));
    assert!(matches!(app.overlay, Overlay::None));
}

#[test]
fn opening_a_vault_file_raises_the_overlay_locked() {
    let dir = TempDir::new().expect("temp dir");
    let path = dir.path().join("vault.kdbx");
    File::create(&path).expect("create vault");
    let mut app = app_showing(&dir);

    assert!(open_vault_if_kdbx(&mut app, &path));

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
fn enter_opens_the_vault_instead_of_the_panel_default() {
    let dir = TempDir::new().expect("temp dir");
    File::create(dir.path().join("vault.kdbx")).expect("create vault");
    let mut app = app_showing(&dir);
    select(&mut app, "vault.kdbx");
    let before = app.left_panel.path.clone();

    handle_open(&mut app).expect("open");

    assert!(app.kdbx_session().is_some());
    assert_eq!(
        app.left_panel.path, before,
        "the panel must not also navigate"
    );
}

#[test]
fn enter_still_descends_into_directories() {
    let dir = TempDir::new().expect("temp dir");
    fs::create_dir(dir.path().join("plain")).expect("mkdir");
    let mut app = app_showing(&dir);
    select(&mut app, "plain");

    handle_open(&mut app).expect("open");

    assert!(matches!(app.overlay, Overlay::None));
    assert_eq!(app.left_panel.path, dir.path().join("plain"));
}

#[test]
fn f3_views_a_vault_through_the_same_trigger_as_enter() {
    let dir = TempDir::new().expect("temp dir");
    File::create(dir.path().join("vault.kdbx")).expect("create vault");
    let mut app = app_showing(&dir);
    select(&mut app, "vault.kdbx");

    crate::ops::viewer::handle_view(&mut app).expect("view");

    assert!(
        app.kdbx_session().is_some(),
        "F3 must open the vault browser, not the plain viewer"
    );
}
