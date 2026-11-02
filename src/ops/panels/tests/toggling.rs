//! Purpose: Hiding and showing a panel, the cursor it leaves, and the menu tick.

use super::*;

#[test]
fn toggle_hides_a_panel_and_reports_it() {
    let mut app = app();
    toggle(&mut app, PanelSide::Right);
    assert_eq!(app.panels_shown, PanelsShown::LeftOnly);
    assert_eq!(app.status.to_string(), "Right panel off");
}

#[test]
fn toggle_twice_brings_the_panel_back() {
    let mut app = app();
    toggle(&mut app, PanelSide::Left);
    toggle(&mut app, PanelSide::Left);
    assert_eq!(app.panels_shown, PanelsShown::Both);
    assert_eq!(app.status.to_string(), "Left panel on");
}

#[test]
fn hiding_the_active_panel_moves_the_cursor_to_the_other() {
    let mut app = app();
    app.active_panel = PanelSide::Left;
    toggle(&mut app, PanelSide::Left);
    assert_eq!(app.panels_shown, PanelsShown::RightOnly);
    assert_eq!(app.active_panel, PanelSide::Right);
}

#[test]
fn hiding_the_inactive_panel_keeps_the_cursor_where_it_was() {
    let mut app = app();
    app.active_panel = PanelSide::Left;
    toggle(&mut app, PanelSide::Right);
    assert_eq!(app.panels_shown, PanelsShown::LeftOnly);
    assert_eq!(app.active_panel, PanelSide::Left);
}

/// Far lets Ctrl+F1 then Ctrl+F2 leave no panel on screen (the console shows
/// through); this program keeps the last one, since the freed screen has no
/// console worth looking at, and says so instead of pretending the key did
/// something.
#[test]
fn the_last_panel_cannot_be_hidden() {
    let mut app = app();
    toggle(&mut app, PanelSide::Right);
    toggle(&mut app, PanelSide::Left);
    assert_eq!(app.panels_shown, PanelsShown::LeftOnly);
    assert_eq!(app.active_panel, PanelSide::Left);
    assert_eq!(
        app.status.to_string(),
        "Left panel stays on: the last panel cannot be hidden (Ctrl+O hides both)"
    );
}

/// Far ignores Tab while a panel is hidden: the cursor would otherwise sit on
/// files nobody can see.
#[test]
fn tab_does_not_reach_a_hidden_panel() {
    let mut app = app();
    app.active_panel = PanelSide::Left;
    toggle(&mut app, PanelSide::Right);
    switch_panel(&mut app);
    assert_eq!(app.active_panel, PanelSide::Left);
    toggle(&mut app, PanelSide::Right);
    switch_panel(&mut app);
    assert_eq!(
        app.active_panel,
        PanelSide::Right,
        "Tab works again once both are shown"
    );
}

#[test]
fn left_and_right_keys_select_only_a_visible_panel() {
    let mut app = app();
    app.active_panel = PanelSide::Right;
    toggle(&mut app, PanelSide::Left);
    select(&mut app, PanelSide::Left);
    assert_eq!(
        app.active_panel,
        PanelSide::Right,
        "the left panel is off screen"
    );
    select(&mut app, PanelSide::Right);
    assert_eq!(app.active_panel, PanelSide::Right);
    toggle(&mut app, PanelSide::Left);
    select(&mut app, PanelSide::Left);
    assert_eq!(app.active_panel, PanelSide::Left);
}

/// The menu's "Panel On/Off" tick follows the toggle for the heading's side.
#[test]
fn the_menu_tick_follows_the_toggle() {
    let mut app = app();
    toggle(&mut app, PanelSide::Right);
    assert!(!panel_on_ticked(&app, Heading::Right));
    assert!(panel_on_ticked(&app, Heading::Left));
    toggle(&mut app, PanelSide::Right);
    assert!(panel_on_ticked(&app, Heading::Right));
}
