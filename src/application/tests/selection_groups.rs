//! Unit tests: Far's group selection keys, and F9 as a toggle.
//!
//! Gray + / Gray − / Gray ∗ / Ctrl+M are the keys a Far user reaches for
//! dozens of times an hour, and every one of them changes a set that the next
//! F5 or F8 acts on — so the tests here are about *which files end up marked*,
//! not about which dialog appeared.

use std::fs;

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use tempfile::TempDir;

use crate::action::Action;
use crate::mask::Mask;
use crate::models::{App, MaskPurpose, Overlay, Panel};
use crate::popup::process_popup_input;
use crate::test_support::{key_event, panels_action as resolve};

mod dialog_and_f9;

/// An app whose active panel lists `names` as ordinary files.
fn app_over(names: &[&str]) -> (App, TempDir) {
    let dir = TempDir::new().expect("test setup");
    for name in names {
        fs::write(dir.path().join(name), b"x").expect("test setup");
    }
    let mut app = App::new().expect("test setup");
    app.left_panel = Panel::new(dir.path().to_path_buf()).expect("test setup");
    app.right_panel = Panel::new(dir.path().to_path_buf()).expect("test setup");
    (app, dir)
}

/// What is marked, in listing order.
fn marked(app: &mut App) -> Vec<String> {
    let mut names: Vec<String> = app
        .get_active_panel()
        .marked
        .iter()
        .map(|name| name.as_str().to_owned())
        .collect();
    names.sort();
    names
}

const FILES: [&str; 5] = ["a.rs", "b.rs", "c.txt", "d.txt", "e.md"];

// ─── the keys reach the actions ─────────────────────────────────────────────

/// Terminals cannot say which keypad a key came from, so Far's grey keys are
/// the plain characters here. They must not fall through to the command line.
#[test]
fn the_group_characters_are_selection_keys_rather_than_command_text() {
    let plain = |code| resolve(KeyEvent::new(code, KeyModifiers::empty()));
    assert_eq!(plain(KeyCode::Char('+')), Action::SelectGroup);
    assert_eq!(plain(KeyCode::Char('-')), Action::UnselectGroup);
    assert_eq!(plain(KeyCode::Char('*')), Action::InvertSelection);
    assert_eq!(
        resolve(KeyEvent::new(KeyCode::Char('m'), KeyModifiers::CONTROL)),
        Action::RestoreSelection
    );
}

// ─── the mask does what it says ─────────────────────────────────────────────

#[test]
fn selecting_by_mask_marks_exactly_the_matching_rows() {
    let (mut app, _dir) = app_over(&FILES);
    let changed = app
        .get_active_panel()
        .mark_matching(&Mask::parse("*.rs"), true);
    assert_eq!(changed, 2);
    assert_eq!(marked(&mut app), vec!["a.rs", "b.rs"]);
}

/// The `..` row is not a file, and marking it would put it into the set that
/// F8 deletes from.
#[test]
fn the_parent_row_can_never_be_selected_by_a_mask() {
    let (mut app, _dir) = app_over(&FILES);
    app.get_active_panel()
        .mark_matching(&Mask::parse("*"), true);
    assert_eq!(
        marked(&mut app),
        vec!["a.rs", "b.rs", "c.txt", "d.txt", "e.md"]
    );
}

#[test]
fn unselecting_by_mask_takes_only_those_rows_out() {
    let (mut app, _dir) = app_over(&FILES);
    app.get_active_panel()
        .mark_matching(&Mask::parse("*"), true);
    let changed = app
        .get_active_panel()
        .mark_matching(&Mask::parse("*.txt"), false);
    assert_eq!(changed, 2);
    assert_eq!(marked(&mut app), vec!["a.rs", "b.rs", "e.md"]);
}

/// The count is what the status line reports, so it must be the number of rows
/// that *changed* rather than the number the mask matched.
#[test]
fn the_count_is_of_rows_that_changed_not_of_rows_that_matched() {
    let (mut app, _dir) = app_over(&FILES);
    assert_eq!(
        app.get_active_panel()
            .mark_matching(&Mask::parse("*.rs"), true),
        2
    );
    assert_eq!(
        app.get_active_panel()
            .mark_matching(&Mask::parse("*.rs"), true),
        0,
        "already selected"
    );
}

#[test]
fn inverting_swaps_the_selected_for_the_unselected() {
    let (mut app, _dir) = app_over(&FILES);
    app.get_active_panel()
        .mark_matching(&Mask::parse("*.rs"), true);
    app.get_active_panel().invert_marks();
    assert_eq!(marked(&mut app), vec!["c.txt", "d.txt", "e.md"]);
    app.get_active_panel().invert_marks();
    assert_eq!(marked(&mut app), vec!["a.rs", "b.rs"], "twice is a no-op");
}

/// The key exists to save a user who has just typed a wrong mask over a
/// careful hand-made selection.
#[test]
fn restoring_puts_back_the_selection_the_last_mask_replaced() {
    let (mut app, _dir) = app_over(&FILES);
    app.get_active_panel()
        .mark_matching(&Mask::parse("*.rs"), true);
    app.get_active_panel()
        .mark_matching(&Mask::parse("*.md"), true);
    assert_eq!(marked(&mut app), vec!["a.rs", "b.rs", "e.md"]);

    app.get_active_panel().restore_marks();
    assert_eq!(marked(&mut app), vec!["a.rs", "b.rs"]);
}

/// …and pressing it twice returns to where it started, which is what a user
/// who pressed it by mistake expects.
#[test]
fn restoring_twice_returns_to_where_it_began() {
    let (mut app, _dir) = app_over(&FILES);
    app.get_active_panel()
        .mark_matching(&Mask::parse("*.rs"), true);
    app.get_active_panel().invert_marks();
    let inverted = marked(&mut app);

    app.get_active_panel().restore_marks();
    assert_ne!(marked(&mut app), inverted);
    app.get_active_panel().restore_marks();
    assert_eq!(marked(&mut app), inverted);
}
