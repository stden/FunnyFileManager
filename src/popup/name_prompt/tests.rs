//! Unit tests: Alt+F6 from the keys down to the link on disk.
//!
//! Driven through `process_popup_input`, the entry point the event loop uses,
//! so the dialog's keys and the filesystem work they end in are one path.

use std::fs;

use crossterm::event::KeyCode;
use tempfile::TempDir;

use crate::models::{App, LinkType, NameField, Overlay, Panel, PanelSide};
use crate::popup::process_popup_input;
use crate::test_support::key_event;

/// Left panel on `source` (holding `entries`; a trailing `/` makes a
/// directory), right panel on `other`, the cursor on the first entry, and the
/// Link dialog open.
fn linking(entries: &[&str]) -> (App, TempDir) {
    let root = TempDir::new().expect("test setup");
    let source = root.path().join("source");
    let other = root.path().join("other");
    fs::create_dir_all(&source).expect("test setup");
    fs::create_dir_all(&other).expect("test setup");
    for entry in entries {
        match entry.strip_suffix('/') {
            Some(dir) => fs::create_dir(source.join(dir)).expect("test setup"),
            None => fs::write(source.join(entry), b"payload").expect("test setup"),
        }
    }
    let mut app = App::new().expect("test setup");
    app.left_panel = Panel::new(source).expect("test setup");
    app.right_panel = Panel::new(other).expect("test setup");
    app.active_panel = PanelSide::Left;
    app.left_panel
        .select_named(entries[0].trim_end_matches('/'));
    crate::ops::link::open(&mut app);
    (app, root)
}

fn press(app: &mut App, keys: &[KeyCode]) {
    for code in keys {
        process_popup_input(app, key_event(*code));
    }
}

fn link_kind(app: &App) -> LinkType {
    let Overlay::NamePrompt(prompt) = &app.overlay else {
        panic!("the dialog is open");
    };
    prompt.link()
}

fn current_field(app: &App) -> NameField {
    let Overlay::NamePrompt(prompt) = &app.overlay else {
        panic!("the dialog is open");
    };
    prompt.field()
}

/// The whole point of the key: Alt+F6, Enter, and a link to the file under
/// the cursor stands in the other panel.
#[cfg(unix)]
#[test]
fn enter_makes_a_symbolic_link_in_the_other_panel() {
    let (mut app, root) = linking(&["notes.txt"]);
    press(&mut app, &[KeyCode::Enter]);

    assert!(!app.overlay.is_active(), "the dialog closed");
    let link = root.path().join("other").join("notes.txt");
    assert_eq!(
        fs::read_link(&link).expect("a link"),
        root.path().join("source").join("notes.txt")
    );
}

/// Right on the drop-down chooses a hard link for a file, and Enter makes
/// one — no "none" in the list, because a link that is not a link is nothing.
#[test]
fn right_on_the_drop_down_offers_a_hard_link_for_a_file() {
    let (mut app, root) = linking(&["notes.txt"]);
    press(&mut app, &[KeyCode::Down]);
    assert_eq!(current_field(&app), NameField::Link);
    press(&mut app, &[KeyCode::Right]);
    assert_eq!(link_kind(&app), LinkType::Hard);
    press(&mut app, &[KeyCode::Right]);
    assert_eq!(link_kind(&app), LinkType::Symbolic, "two choices, wrapping");
    press(&mut app, &[KeyCode::Left]);
    assert_eq!(link_kind(&app), LinkType::Hard);

    press(&mut app, &[KeyCode::Enter]);
    let link = root.path().join("other").join("notes.txt");
    assert!(link.is_file(), "a hard link is a file");
    assert!(!fs::symlink_metadata(&link).expect("metadata").is_symlink());
}

/// For a folder the drop-down has one answer. Far does not list hard links
/// for a directory, and neither arrow may reach what is not listed.
#[test]
fn the_drop_down_never_leaves_symbolic_for_a_folder() {
    let (mut app, _root) = linking(&["photos/"]);
    press(&mut app, &[KeyCode::Down]);
    assert_eq!(current_field(&app), NameField::Link);
    for code in [KeyCode::Right, KeyCode::Left, KeyCode::Right] {
        press(&mut app, &[code]);
        assert_eq!(link_kind(&app), LinkType::Symbolic, "{code:?}");
    }
}

#[test]
fn escape_makes_nothing() {
    let (mut app, root) = linking(&["notes.txt"]);
    press(&mut app, &[KeyCode::Esc]);
    assert!(!app.overlay.is_active());
    assert_eq!(
        fs::read_dir(root.path().join("other"))
            .expect("read")
            .count(),
        0
    );
}

/// The name field is a path, and typing into it changes where the link goes.
#[cfg(unix)]
#[test]
fn the_name_typed_is_where_the_link_goes() {
    let (mut app, root) = linking(&["notes.txt"]);
    // `notes.txt` -> `notes.` -> `notes.md`.
    for _ in 0..3 {
        press(&mut app, &[KeyCode::Backspace]);
    }
    press(
        &mut app,
        &[KeyCode::Char('m'), KeyCode::Char('d'), KeyCode::Enter],
    );

    assert!(root.path().join("other").join("notes.md").is_symlink());
}

/// Ctrl+letter is a shortcut, not text — the name field must not swallow it.
#[test]
fn a_control_key_is_not_typed_into_the_name() {
    use crate::test_support::key_event_with_mods;
    use crossterm::event::KeyModifiers;

    let (mut app, _root) = linking(&["notes.txt"]);
    let Overlay::NamePrompt(before) = app.overlay.clone() else {
        panic!("open");
    };
    process_popup_input(
        &mut app,
        key_event_with_mods(KeyCode::Char('x'), KeyModifiers::CONTROL),
    );
    let Overlay::NamePrompt(after) = &app.overlay else {
        panic!("still open");
    };
    assert_eq!(after.name(), before.name());
}
