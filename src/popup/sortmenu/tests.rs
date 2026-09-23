//! Unit tests: the keys of the Sort by menu, against a real directory so a
//! choice shows in the listing and not only in a flag.

use std::fs;

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use tempfile::TempDir;

use crate::models::{App, Overlay, Panel, SortMode, SortRow};
use crate::popup::process_popup_input;
use crate::popup::tests::key;

fn app_over(temp: &TempDir) -> App {
    fs::write(temp.path().join("a.txt"), "aaaaaaaaaa").expect("test setup");
    fs::write(temp.path().join("b.txt"), "b").expect("test setup");
    let left = Panel::new(temp.path().to_path_buf()).expect("test setup");
    let right = Panel::new(temp.path().to_path_buf()).expect("test setup");
    App::with_panels(left, right)
}

fn cursor_row(app: &App) -> SortRow {
    let Overlay::SortModes(menu) = &app.overlay else {
        panic!("the sort menu is open");
    };
    menu.row()
}

fn files(app: &App) -> Vec<String> {
    app.get_active_panel_ref()
        .entries
        .iter()
        .filter(|entry| !entry.kind.is_dir())
        .map(|entry| entry.name.clone())
        .collect()
}

/// Opening sets the variant; Esc takes it down and runs nothing else.
#[test]
fn open_sets_the_variant_and_esc_closes_it() {
    let mut app = App::new().expect("test setup");
    crate::ops::sortmenu::open(&mut app);
    assert!(matches!(app.overlay, Overlay::SortModes(_)));

    process_popup_input(&mut app, key(KeyCode::Char('q')));
    assert!(
        matches!(app.overlay, Overlay::SortModes(_)),
        "letters that are not a row's hotkey leave the menu open"
    );

    process_popup_input(&mut app, key(KeyCode::Esc));
    assert!(!app.overlay.is_active());
    assert_eq!(app.get_active_panel_ref().sort_mode, SortMode::Name);
}

#[test]
fn f10_closes_it_too() {
    let mut app = App::new().expect("test setup");
    crate::ops::sortmenu::open(&mut app);
    process_popup_input(&mut app, key(KeyCode::F(10)));
    assert!(!app.overlay.is_active());
}

#[test]
fn process_ignores_a_closed_menu_and_ctrl_f_keys_choose_a_row() {
    let mut app = App::new().expect("test setup");
    assert!(!super::process(&mut app, KeyCode::Esc, KeyModifiers::NONE));
    crate::ops::sortmenu::open(&mut app);
    process_popup_input(
        &mut app,
        KeyEvent::new(KeyCode::F(6), KeyModifiers::CONTROL),
    );
    assert!(!app.overlay.is_active());
    assert_eq!(app.get_active_panel_ref().sort_mode, SortMode::Size);
}

#[test]
fn an_unbound_key_leaves_the_sort_menu_open() {
    let mut app = App::new().expect("test setup");
    crate::ops::sortmenu::open(&mut app);
    process_popup_input(&mut app, key(KeyCode::Insert));
    assert!(matches!(app.overlay, Overlay::SortModes(_)));
}

#[test]
fn the_arrows_move_the_cursor_and_home_end_jump() {
    let mut app = App::new().expect("test setup");
    crate::ops::sortmenu::open(&mut app);
    assert_eq!(cursor_row(&app), SortRow::Mode(SortMode::Name));

    process_popup_input(&mut app, key(KeyCode::Down));
    assert_eq!(cursor_row(&app), SortRow::Mode(SortMode::Extension));
    process_popup_input(&mut app, key(KeyCode::Up));
    process_popup_input(&mut app, key(KeyCode::Up));
    assert_eq!(cursor_row(&app), SortRow::Reverse, "wraps from the top");
    process_popup_input(&mut app, key(KeyCode::Home));
    assert_eq!(cursor_row(&app), SortRow::Mode(SortMode::Name));
    process_popup_input(&mut app, key(KeyCode::End));
    assert_eq!(cursor_row(&app), SortRow::Reverse);
}

#[test]
fn enter_on_a_mode_sets_it_and_closes_the_menu() {
    let temp = TempDir::new().expect("test setup");
    let mut app = app_over(&temp);
    crate::ops::sortmenu::open(&mut app);
    for _ in 0..3 {
        process_popup_input(&mut app, key(KeyCode::Down));
    }
    assert_eq!(cursor_row(&app), SortRow::Mode(SortMode::Size));

    process_popup_input(&mut app, key(KeyCode::Enter));

    assert!(!app.overlay.is_active());
    assert_eq!(app.get_active_panel_ref().sort_mode, SortMode::Size);
    assert_eq!(files(&app), ["a.txt", "b.txt"], "largest first");
}

/// Far: the menu opens on the current mode, so Enter alone reverses it.
#[test]
fn enter_on_the_current_mode_reverses_the_order() {
    let temp = TempDir::new().expect("test setup");
    let mut app = app_over(&temp);
    crate::ops::sortmenu::open(&mut app);

    process_popup_input(&mut app, key(KeyCode::Enter));

    assert!(!app.overlay.is_active());
    assert!(app.get_active_panel_ref().sort_reversed);
    assert_eq!(files(&app), ["b.txt", "a.txt"]);
}

#[test]
fn enter_on_the_reverse_row_flips_the_order() {
    let temp = TempDir::new().expect("test setup");
    let mut app = app_over(&temp);
    crate::ops::sortmenu::open(&mut app);
    process_popup_input(&mut app, key(KeyCode::End));

    process_popup_input(&mut app, key(KeyCode::Enter));

    assert!(!app.overlay.is_active());
    let panel = app.get_active_panel_ref();
    assert_eq!(panel.sort_mode, SortMode::Name, "the mode is untouched");
    assert!(panel.sort_reversed);
}

/// A hotkey picks its row and runs it, as Far's menus do.
#[test]
fn a_hotkey_letter_chooses_its_row_at_once() {
    let temp = TempDir::new().expect("test setup");
    let mut app = app_over(&temp);
    crate::ops::sortmenu::open(&mut app);

    process_popup_input(&mut app, key(KeyCode::Char('S')));

    assert!(!app.overlay.is_active());
    assert_eq!(app.get_active_panel_ref().sort_mode, SortMode::Size);
    assert!(!app.get_active_panel_ref().sort_reversed);
}

#[test]
fn a_letter_that_names_no_row_is_consumed_and_changes_nothing() {
    let mut app = App::new().expect("test setup");
    crate::ops::sortmenu::open(&mut app);
    process_popup_input(&mut app, key(KeyCode::Char('q')));
    assert!(matches!(app.overlay, Overlay::SortModes(_)));
    assert_eq!(cursor_row(&app), SortRow::Mode(SortMode::Name));
}

/// Alt+letter is quick search's chord, not a menu hotkey; it must not pick a
/// row the user did not mean.
#[test]
fn a_letter_with_alt_or_ctrl_is_not_a_hotkey() {
    let mut app = App::new().expect("test setup");
    crate::ops::sortmenu::open(&mut app);
    process_popup_input(
        &mut app,
        KeyEvent::new(KeyCode::Char('s'), KeyModifiers::ALT),
    );
    assert!(matches!(app.overlay, Overlay::SortModes(_)));
    assert_eq!(app.get_active_panel_ref().sort_mode, SortMode::Name);
}

/// Far's `+`, `-` and `*`: direct, reversed, and flipped — each closes the menu.
#[test]
fn plus_minus_and_star_set_the_order_and_close() {
    let temp = TempDir::new().expect("test setup");
    let mut app = app_over(&temp);

    crate::ops::sortmenu::open(&mut app);
    process_popup_input(&mut app, key(KeyCode::Char('-')));
    assert!(!app.overlay.is_active());
    assert!(app.get_active_panel_ref().sort_reversed);
    assert_eq!(files(&app), ["b.txt", "a.txt"]);

    crate::ops::sortmenu::open(&mut app);
    process_popup_input(&mut app, key(KeyCode::Char('*')));
    assert!(!app.overlay.is_active());
    assert!(!app.get_active_panel_ref().sort_reversed);

    crate::ops::sortmenu::open(&mut app);
    process_popup_input(&mut app, key(KeyCode::Char('-')));
    crate::ops::sortmenu::open(&mut app);
    // Shift+= is how most terminals deliver `+`; the modifier is not a veto.
    process_popup_input(
        &mut app,
        KeyEvent::new(KeyCode::Char('+'), KeyModifiers::SHIFT),
    );
    assert!(!app.overlay.is_active());
    assert!(!app.get_active_panel_ref().sort_reversed);
    assert_eq!(files(&app), ["a.txt", "b.txt"]);
}
