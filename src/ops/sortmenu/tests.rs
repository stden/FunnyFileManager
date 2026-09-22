//! Unit tests: the sort menu opens on the panel's mode, and every way of
//! choosing — a row, a direct key, the order keys — re-lists a real directory.

use std::fs;

use tempfile::TempDir;

use super::{choose, open, order, set, toggle_order};
use crate::models::{App, Overlay, Panel, SortMode, SortOrder, SortRow};
use crate::status::Status;

/// Three files whose name order and size order disagree, so a listing says
/// which mode built it.
fn app_over(temp: &TempDir) -> App {
    fs::write(temp.path().join("a.txt"), "aaaaaaaaaa").expect("test setup");
    fs::write(temp.path().join("b.txt"), "b").expect("test setup");
    fs::write(temp.path().join("c.txt"), "cccc").expect("test setup");
    let left = Panel::new(temp.path().to_path_buf()).expect("test setup");
    let right = Panel::new(temp.path().to_path_buf()).expect("test setup");
    App::with_panels(left, right)
}

fn files(app: &App) -> Vec<String> {
    app.get_active_panel_ref()
        .entries
        .iter()
        .filter(|entry| !entry.kind.is_dir())
        .map(|entry| entry.name.clone())
        .collect()
}

#[test]
fn opening_puts_the_cursor_and_the_tick_on_the_panels_mode() {
    let temp = TempDir::new().expect("test setup");
    let mut app = app_over(&temp);
    app.get_active_panel().sort_mode = SortMode::Size;
    app.get_active_panel().sort_reversed = true;

    open(&mut app);

    let Overlay::SortModes(menu) = &app.overlay else {
        panic!("the sort menu is open");
    };
    assert_eq!(menu.row(), SortRow::Mode(SortMode::Size));
    assert!(menu.is_ticked(SortRow::Mode(SortMode::Size)));
    assert!(menu.is_ticked(SortRow::Reverse));
}

#[test]
fn a_direct_key_sets_a_new_mode_in_direct_order() {
    let temp = TempDir::new().expect("test setup");
    let mut app = app_over(&temp);
    app.get_active_panel().sort_reversed = true;

    set(&mut app, SortMode::Size);

    let panel = app.get_active_panel_ref();
    assert_eq!(panel.sort_mode, SortMode::Size);
    assert!(!panel.sort_reversed, "a new mode starts the direct way");
    assert_eq!(files(&app), ["a.txt", "c.txt", "b.txt"], "largest first");
    assert_eq!(app.status.to_string(), "Sort: Size");
}

/// Far: the key of the mode already in force reverses it.
#[test]
fn the_same_key_again_reverses_and_the_listing_turns_around() {
    let temp = TempDir::new().expect("test setup");
    let mut app = app_over(&temp);
    assert_eq!(files(&app), ["a.txt", "b.txt", "c.txt"]);

    set(&mut app, SortMode::Name);
    assert!(app.get_active_panel_ref().sort_reversed);
    assert_eq!(files(&app), ["c.txt", "b.txt", "a.txt"]);
    assert_eq!(app.status.to_string(), "Sort: Name, reverse order");

    set(&mut app, SortMode::Name);
    assert!(!app.get_active_panel_ref().sort_reversed, "and back");
    assert_eq!(files(&app), ["a.txt", "b.txt", "c.txt"]);
}

#[test]
fn choosing_the_reverse_row_flips_the_order_and_keeps_the_mode() {
    let temp = TempDir::new().expect("test setup");
    let mut app = app_over(&temp);
    app.get_active_panel().sort_mode = SortMode::Size;

    choose(&mut app, SortRow::Reverse);

    let panel = app.get_active_panel_ref();
    assert_eq!(panel.sort_mode, SortMode::Size);
    assert!(panel.sort_reversed);
    assert_eq!(files(&app), ["b.txt", "c.txt", "a.txt"], "smallest first");
}

#[test]
fn choosing_a_mode_row_is_the_direct_key() {
    let temp = TempDir::new().expect("test setup");
    let mut app = app_over(&temp);

    choose(&mut app, SortRow::Mode(SortMode::Extension));
    assert_eq!(app.get_active_panel_ref().sort_mode, SortMode::Extension);

    choose(&mut app, SortRow::Mode(SortMode::Extension));
    assert!(
        app.get_active_panel_ref().sort_reversed,
        "the same row reverses"
    );
}

/// `+` and `-` are absolute: pressing `-` twice leaves the order reversed.
#[test]
fn the_order_keys_set_an_order_rather_than_toggling_it() {
    let temp = TempDir::new().expect("test setup");
    let mut app = app_over(&temp);

    order(&mut app, SortOrder::Reversed);
    order(&mut app, SortOrder::Reversed);
    assert!(app.get_active_panel_ref().sort_reversed);
    assert_eq!(files(&app), ["c.txt", "b.txt", "a.txt"]);

    order(&mut app, SortOrder::Direct);
    assert!(!app.get_active_panel_ref().sort_reversed);
    assert_eq!(files(&app), ["a.txt", "b.txt", "c.txt"]);
}

#[test]
fn the_star_key_toggles() {
    let temp = TempDir::new().expect("test setup");
    let mut app = app_over(&temp);

    toggle_order(&mut app);
    assert!(app.get_active_panel_ref().sort_reversed);
    toggle_order(&mut app);
    assert!(!app.get_active_panel_ref().sort_reversed);
}

/// Directories stay on top when the order is reversed, as in Far: reversing
/// the listing is not the same as turning it upside down.
#[test]
fn reversing_keeps_directories_above_the_files() {
    let temp = TempDir::new().expect("test setup");
    fs::create_dir(temp.path().join("zdir")).expect("test setup");
    let mut app = app_over(&temp);

    order(&mut app, SortOrder::Reversed);

    let names: Vec<&str> = app
        .get_active_panel_ref()
        .entries
        .iter()
        .map(|entry| entry.name.as_str())
        .collect();
    let zdir = names
        .iter()
        .position(|name| *name == "zdir")
        .expect("listed");
    let a = names
        .iter()
        .position(|name| *name == "a.txt")
        .expect("listed");
    assert!(zdir < a, "{names:?}");
}

/// A directory that vanished under the panel cannot be re-listed; the panel
/// keeps its old entries and the status bar says why.
#[test]
fn a_failed_relist_is_reported_not_swallowed() {
    let temp = TempDir::new().expect("test setup");
    let mut app = app_over(&temp);
    let before = files(&app);
    fs::remove_dir_all(temp.path()).expect("test setup");

    set(&mut app, SortMode::Size);

    assert!(
        matches!(app.status, Status::Error(_)),
        "an error, not a sort status: {}",
        app.status
    );
    assert!(app.lifecycle.is_running(), "a lost directory is not fatal");
    assert_eq!(files(&app), before, "the old listing survives");
}
