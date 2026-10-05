//! Purpose: the sort menu's hotkeys — a letter chooses its row at once, an unknown or modified letter does nothing, and + - * set the order.

use super::*;

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
