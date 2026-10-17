//! Purpose: the F9 menu bar's keys — walking the headings and rows, letters that pick a row, and what Enter and Esc hand on.

use super::*;

// ─── the menu bar ───────────────────────────────────────────────────────────

fn open_menu(app: &mut App, heading: Heading) {
    app.set_menu(Menu::open(heading));
}

fn menu_of(app: &App) -> Menu {
    let Overlay::Menu(menu) = &app.overlay else {
        panic!("the bar is open");
    };
    *menu
}

#[test]
fn left_and_right_walk_the_bar_in_both_directions() {
    let (mut app, _dir) = app_over(&[]);
    open_menu(&mut app, Heading::Files);

    press(&mut app, KeyCode::Left);
    assert_eq!(menu_of(&app).heading, Heading::Left);
    press(&mut app, KeyCode::Right);
    assert_eq!(menu_of(&app).heading, Heading::Files);
}

/// Walking the bar past a panel's heading only shows its menu: the active
/// panel stays put, so a Files or Commands row still acts on the panel the
/// user was in when F9 was pressed, and Esc leaves everything as it was.
#[test]
fn walking_across_a_panel_heading_keeps_the_active_panel() {
    let (mut app, _dir) = app_over(&[]);
    app.active_panel = PanelSide::Right;
    open_menu(&mut app, Heading::Right);

    press(&mut app, KeyCode::Right);
    assert_eq!(menu_of(&app).heading, Heading::Left);
    press(&mut app, KeyCode::Right);
    assert_eq!(menu_of(&app).heading, Heading::Files);
    assert_eq!(app.active_panel, PanelSide::Right, "walked past Left");

    press(&mut app, KeyCode::Left);
    press(&mut app, KeyCode::Esc);
    assert_eq!(
        app.active_panel,
        PanelSide::Right,
        "Esc on the Left heading"
    );
}

/// A row of the Left or Right menu acts on its own panel. The keys behind
/// those rows follow the active panel, so choosing one — and only choosing
/// one — makes that panel active.
#[test]
fn choosing_a_row_under_a_panel_heading_makes_that_panel_active() {
    let (mut app, _dir) = app_over(&[]);
    app.active_panel = PanelSide::Left;
    open_menu(&mut app, Heading::Right);
    assert_eq!(app.active_panel, PanelSide::Left, "only the menu is open");

    press(&mut app, KeyCode::Enter);

    assert!(!app.overlay.is_active());
    assert!(app.pending_row.is_some(), "the row is handed on");
    assert_eq!(app.active_panel, PanelSide::Right);
}

#[test]
fn up_and_down_walk_the_rows() {
    let (mut app, _dir) = app_over(&[]);
    open_menu(&mut app, Heading::Files);
    let first = menu_of(&app).row;

    press(&mut app, KeyCode::Down);
    let second = menu_of(&app).row;
    assert_ne!(second, first);
    press(&mut app, KeyCode::Up);
    assert_eq!(menu_of(&app).row, first);
}

/// Enter hands the chosen row's key on, and the bar leaves the screen first so
/// a command that opens a dialog is not fighting the drop-down for it.
#[test]
fn enter_closes_the_bar_and_hands_on_the_rows_key() {
    let (mut app, _dir) = app_over(&["a.txt"]);
    open_menu(&mut app, Heading::Files);

    press(&mut app, KeyCode::Enter);

    assert!(!app.overlay.is_active(), "the bar left the screen first");
    assert!(app.pending_row.is_some(), "and the row was handed on");
}

#[test]
fn a_key_the_bar_has_no_use_for_leaves_it_open() {
    let (mut app, _dir) = app_over(&[]);
    open_menu(&mut app, Heading::Files);
    press(&mut app, KeyCode::Insert);
    assert!(matches!(app.overlay, Overlay::Menu(_)), "still open");
}

/// A letter picks a row here; a letter this drop-down has no row for walks the
/// bar instead, which is how Far lets the bar be driven by its letters
/// whichever menu happens to be open.
#[test]
fn a_letter_picks_a_row_or_moves_to_the_heading_that_owns_it() {
    let (mut app, _dir) = app_over(&["a.txt"]);
    open_menu(&mut app, Heading::Files);
    press(&mut app, KeyCode::Char('d'));
    assert!(!app.overlay.is_active(), "Delete ran");

    // Files has no row on L, so it falls through to the Left heading.
    let (mut app, _dir) = app_over(&[]);
    open_menu(&mut app, Heading::Files);
    press(&mut app, KeyCode::Char('l'));
    assert_eq!(
        menu_of(&app).heading,
        Heading::Left,
        "moved to the Left menu"
    );

    let (mut app, _dir) = app_over(&[]);
    open_menu(&mut app, Heading::Files);
    press(&mut app, KeyCode::Char('9'));
    assert!(matches!(app.overlay, Overlay::Menu(_)), "no such letter");
}

#[test]
fn escape_closes_the_bar_and_runs_nothing() {
    let (mut app, _dir) = app_over(&[]);
    open_menu(&mut app, Heading::Files);
    press(&mut app, KeyCode::Esc);
    assert!(!app.overlay.is_active());
    assert!(app.pending_row.is_none());
    assert!(app.lifecycle.is_running(), "{:?}", Lifecycle::Running);
}
