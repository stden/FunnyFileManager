//! Unit tests: what F7 actually creates, and where it leaves the cursor.
//!
//! Driven through `process_popup_input`, which is the same entry point the
//! event loop uses — the dialog's keys and the filesystem work it ends in are
//! one path, and testing them apart would leave the join untested.

use std::fs;

use crossterm::event::KeyCode;
use tempfile::TempDir;

use crate::models::{App, LinkType, NamePrompt, Overlay, Panel};
use crate::popup::process_popup_input;
use crate::test_support::key_event;

mod links;
mod name_lists;

/// An app with both panels on a temporary directory holding `entries`.
fn app_over(names: &[&str]) -> (App, TempDir) {
    let dir = TempDir::new().expect("test setup");
    for name in names {
        fs::create_dir(dir.path().join(name)).expect("test setup");
    }
    let mut app = App::new().expect("test setup");
    app.left_panel = Panel::new(dir.path().to_path_buf()).expect("test setup");
    app.right_panel = Panel::new(dir.path().to_path_buf()).expect("test setup");
    (app, dir)
}

/// Press each key of `keys` into the open dialog.
fn press(app: &mut App, keys: &[KeyCode]) {
    for code in keys {
        process_popup_input(app, key_event(*code));
    }
}

/// Type a name into whichever field has the cursor.
fn type_name(app: &mut App, text: &str) {
    let keys: Vec<KeyCode> = text.chars().map(KeyCode::Char).collect();
    press(app, &keys);
}

/// The entry the active panel's cursor is on.
fn cursor_on(app: &mut App) -> String {
    app.get_active_panel()
        .get_selected_entry()
        .map(|entry| entry.name.clone())
        .unwrap_or_default()
}

/// The point of the dialog: the folder that was just made is the one the user
/// is about to enter, and hunting for it in a long listing is exactly the work
/// F7 was meant to save.
#[test]
fn the_cursor_lands_on_the_folder_that_was_just_created() {
    let (mut app, dir) = app_over(&["aaa", "zzz"]);
    app.set_overlay(Overlay::NamePrompt(NamePrompt::make_folder()));
    type_name(&mut app, "mmm");
    press(&mut app, &[KeyCode::Enter]);

    assert!(dir.path().join("mmm").is_dir());
    assert_eq!(
        cursor_on(&mut app),
        "mmm",
        "not on the row the cursor happened to be on before"
    );
}

/// …and it lands there however the listing is sorted, which a test that only
/// ever creates the last name would not catch.
#[test]
fn the_cursor_finds_the_new_folder_wherever_the_sort_puts_it() {
    for name in ["000_first", "middle", "zzz_last"] {
        let (mut app, _dir) = app_over(&["alpha", "omega"]);
        app.set_overlay(Overlay::NamePrompt(NamePrompt::make_folder()));
        type_name(&mut app, name);
        press(&mut app, &[KeyCode::Enter]);
        assert_eq!(cursor_on(&mut app), name);
    }
}

#[test]
fn a_name_already_taken_is_refused_rather_than_overwritten() {
    let (mut app, dir) = app_over(&["taken"]);
    app.set_overlay(Overlay::NamePrompt(NamePrompt::make_folder()));
    type_name(&mut app, "taken");
    press(&mut app, &[KeyCode::Enter]);

    assert!(dir.path().join("taken").is_dir(), "still the original");
    assert!(
        app.status.to_string().contains("already exists"),
        "the status should say why nothing happened, got {}",
        app.status
    );
}

/// Enter on Cancel cancels, wherever else it would mean go.
#[test]
fn enter_on_the_cancel_button_creates_nothing() {
    let (mut app, dir) = app_over(&[]);
    app.set_overlay(Overlay::NamePrompt(NamePrompt::make_folder()));
    type_name(&mut app, "unwanted");
    // Down past the drop-down and the checkbox to OK, then once more.
    press(
        &mut app,
        &[
            KeyCode::Down,
            KeyCode::Down,
            KeyCode::Down,
            KeyCode::Down,
            KeyCode::Enter,
        ],
    );

    assert!(!dir.path().join("unwanted").exists());
    assert!(!app.overlay.is_active(), "and the dialog is gone");
}

#[test]
fn escape_creates_nothing() {
    let (mut app, dir) = app_over(&[]);
    app.set_overlay(Overlay::NamePrompt(NamePrompt::make_folder()));
    type_name(&mut app, "unwanted");
    press(&mut app, &[KeyCode::Esc]);
    assert!(!dir.path().join("unwanted").exists());
}

/// A rename moves the entry in the sort order, and the cursor goes with it —
/// otherwise the file the user just renamed is no longer the one under the
/// cursor, and the next key acts on a stranger.
#[test]
fn the_cursor_follows_a_renamed_entry() {
    let (mut app, dir) = app_over(&["aaa", "bbb", "ccc"]);
    app.get_active_panel().select_named("aaa");
    app.set_overlay(Overlay::NamePrompt(NamePrompt::rename(
        crate::models::FileName::new("aaa"),
    )));
    press(
        &mut app,
        &[KeyCode::Backspace, KeyCode::Backspace, KeyCode::Backspace],
    );
    type_name(&mut app, "zzz");
    press(&mut app, &[KeyCode::Enter]);

    assert!(dir.path().join("zzz").is_dir());
    assert_eq!(cursor_on(&mut app), "zzz");
}

/// F3 on a directory measures it rather than refusing.
///
/// "Cannot view a directory" answered a question nobody asked: the reason to
/// press F3 on a folder is to find out how big it is and what is in it, which
/// is exactly what Ctrl+L already computed for the panel's own path.
#[test]
fn f3_on_a_directory_shows_what_is_in_it() {
    use crate::ops::handle_view;

    let (mut app, dir) = app_over(&["subdir"]);
    fs::write(dir.path().join("subdir/inner.txt"), vec![b'x'; 4096]).expect("test setup");
    app.get_active_panel().load_entries().expect("reload");
    assert!(app.get_active_panel().select_named("subdir"));

    handle_view(&mut app).expect("view");

    let Overlay::Information(stats) = &app.overlay else {
        panic!("F3 on a directory opens the information panel");
    };
    assert_eq!(stats.files, 1, "it counted what is inside");
    assert!(stats.total.bytes() >= 4096, "and measured it");
    assert!(app.get_viewer().is_none(), "no text viewer for a directory");
}
